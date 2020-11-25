//! This crate provides functionality to parse configuration from command line options and
//! optionally an external file.
//!
//! The configuration handling was heavily inspired by the way that ripgrep handles configuration
//! and roughly works as follows:
//! * Options can be specified on the command line
//! * If an environment variable is passed and the value of that variable contains a filename,
//! this file will be parsed as if the content had been specified as command line arguments.
//! Arguments on the command line will take precedence over those loaded from a file.
//!
//! Interaction with this module will be using ConfigDescription and Configuration
//! structs to define the configuration a binary/module needs and then calling get_matcher
//! to parse the command line.
//!
use std::ffi::OsString;
use std::fmt::Error;

use clap::{App, Arg};
use std::collections::{HashMap, HashSet};
use std::env;
use std::hash::{Hash, Hasher};

// Include all "stolen" ripgrep code in this module
mod ripgrep_config;

/// This trait defines the behavior that all configuration classes need to
/// provide in order for the clap matcher to be generated from the config object
trait Configurable {
    /// This method will be called by ConfigBuilder to retrieve an object that describes
    /// the parameters which should be used to parse the command line
    fn get_config_description() -> Configuration;

    /// The parsed command line parameters will be sent to this method
    /// It is the responsibility of the actual implementation to parse the input data
    /// and create a meaningful representation of the data contained in there that
    /// users can then interact with
    ///
    /// * `parsed_values` The values that were parsed from the command line arguments
    /// The keys in the HashMap will be all ConfigOptions that were returned in the
    /// get_config_description() call.
    ///
    /// The value in the HashMap can have three meanings:
    /// - None: this parameter was not specified on the command line
    /// - Some(Vec<String>) with an empty Vector: this is a boolean parameter
    ///   and it was present on the command line
    /// - Some(Vec<String>) with one or more list elements: parameter that takes
    ///   a value and one or more values were specified
    fn parse_values(parsed_values: HashMap<ConfigOption, Option<Vec<String>>>) -> Self;
}

/// This struct describes some properties that can be set for an application as well
/// as the list of options that the program can understand
/// These values (apart from the list of options) are only used to generate the
/// console help message
#[derive(Clone, Debug)]
pub struct Configuration {
    /// The name of the application
    pub name: &'static str,
    /// Version of the application
    pub version: &'static str,
    /// A brief description of what the application does
    pub about: &'static str,
    /// The set of all possible command line options
    /// this is a set instead of a list as we do not want or need duplicates
    pub options: HashSet<ConfigOption>,
}

/// Represents an individual config option that the program can interpret
#[derive(Clone, Debug)]
pub struct ConfigOption {
    /// The name of the option (without leading --)
    pub name: &'static str,
    /// Default value to use for the option if it is not provided
    /// NOTE: this will be ignored if *takes_argument* is true, as
    /// a default value for a switch does not make too much sense
    pub default: Option<&'static str>,
    /// Whether this option has to be provided
    pub required: bool,
    /// If true the option takes a value as argument, if false
    /// the option is a present/missing flag
    pub takes_argument: bool,
    /// Help text to display for the option
    pub help: &'static str,
    /// Longer text to use when generating documentation/website/...
    pub documentation: &'static str,
    /// Allow specifying this argument multiple times?
    /// If true, multiple occurrences of this argument will all be taken into account, if false
    /// only the last occurence will be used, any previous values will be overwritten
    pub list: bool,
}

// Necessary to be able to use a ConfigOption as key in a HashMap
// It is enough to compare the name field for equality, as there is no
// realistic (or useful) scenario where we'd want to support multiple parameters
// with the same name
impl PartialEq for ConfigOption {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

// Necessary to be able to use a ConfigOption as key in a HashMap
impl Eq for ConfigOption {}

// Necessary to be able to use a ConfigOption as key in a HashMap
// This needs to match the equality implementation to avoid collisions/conflicts
// when storing elements as keys in a HashMap
impl Hash for ConfigOption {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

/// A struct that provides associated functions to generate a Clap matcher from a configuration
/// that is described by a struct implementing the Configurable trait.
///
/// The general flow is like this:
/// 1. ConfigBuilder calls the associated function get_config_description on the
/// config object to retrieve the description of the configuration
/// 2. Creates a matcher based on the ConfigOptions from that object
/// 3. Use matcher to parse command line arguments
/// 4. If --no-config parameter was specified return parsed config
/// 5. If --no-config is not present check environment variable STACKABLE_CONFIG_PATH
///    if an external config file is specified
/// 6. Parse config from file and prepend all options to the command line arguments
/// 7. Re-parse combined arguments
/// 8. Call associated function parse_values on config object to create a config object
/// that is populated with proper values based on the parsed argument
/// 9. Return the populated config object
///
/// This effectively means that config can be either provided on the command line, or
/// in a file that is specified via environment variable, with options from the command
/// line taking precedence over the config file.
struct ConfigBuilder {}

impl ConfigBuilder {
    /// The entry point into ConfigBuilder, this method will be called by any binary (or library)
    /// using this lib for command line parsing.
    /// It is a typed method, with the type having to implement the Configurable trait, so the type
    /// has to offer two methods: one to retrieve the definition of command line parameters, one to
    /// pass back the parsed values of those parameters.
    ///
    /// * `commandline` The command line parameters to parse the configuration from (first element will be
    /// ignored, as this is the binary name
    /// * `config_file_env` Name of the environment variable to read an extra config file from
    pub fn build<T: Configurable>(
        commandline: Vec<OsString>,
        config_file_env: &str,
    ) -> Result<T, Error> {
        // Parse commandline according to config definition
        let description = T::get_config_description();

        // Use the command line parameters defined in the description to build a
        // clap matcher object that can be used to parse the acual parameters
        let matcher = ConfigBuilder::create_matcher(&description);

        // Overwrite command line arguments with final arguments to parse
        // if a config file was specified, all options from that file will be
        // prepended to the command line arguments
        let commandline =
            ConfigBuilder::maybe_combine_arguments(matcher.clone(), commandline, config_file_env);

        // Parse command line
        let matcher = matcher.get_matches_from(commandline.expect("Error parsing commandline!"));

        // Convert results from command line parsing into a HashMap<ConfigOption, Vec<String>>
        // this is then passed to the actual implementation of the configuration for processing
        let mut result: HashMap<ConfigOption, Option<Vec<String>>> = HashMap::new();

        for config_option in description.options.clone() {
            if let Some(parsed_values) = matcher.values_of(config_option.name) {
                let parsed_values = parsed_values.collect::<Vec<&str>>();

                // Convert to Vec of owned Strings, as we will want to keep these values around for
                // the lifetime of our application
                let parsed_values: Vec<String> =
                    parsed_values.into_iter().map(String::from).collect();

                result.insert(config_option, Some(parsed_values));
            } else {
                result.insert(config_option, None);
            }
        }
        // Return an actual object of the configuration that is populated with appropriate values
        Ok(T::parse_values(result))
    }

    // Create a clap matcher based on the ConfigOptions that were defined in the config object
    fn create_matcher(config: &Configuration) -> App {
        let mut matches = App::new(config.name)
            .version(config.version)
            .about(config.about);

        for option in config.options.iter() {
            let mut new_arg = Arg::with_name(option.name)
                .long(option.name)
                .value_name(option.name)
                .help(option.help)
                .takes_value(option.takes_argument)
                .required(option.required);

            // Was a default value specified for this option?
            if let Some(default_value) = &option.default {
                // If this is an option that does not take an argument i.e. a switch
                // we ignore any default values that were specified, as these do not really
                // make sense for that
                // If a value is needed in case a switch is specified then this should be handled
                // in the implementing config
                if option.takes_argument {
                    new_arg = new_arg.default_value(default_value);
                }
            }

            if option.list {
                matches = matches.arg(new_arg.multiple(true));
            } else {
                matches = matches.arg(new_arg.overrides_with(option.name));
            }
        }
        matches
    }

    fn maybe_combine_arguments(
        app_matcher: App,
        commandline: Vec<OsString>,
        config_file_env: &str,
    ) -> Result<Vec<OsString>, Error> {
        // Parse provided arguments
        let command_line_args = app_matcher.get_matches_from(&commandline);

        // If --no-config was passed on the command line, we bypass reading values from the
        // extra config file
        let mut args_from_file = if command_line_args.is_present("no-config") {
            vec![]
        } else {
            ripgrep_config::args(config_file_env)
        };

        // Check if there were any arguments in the config file
        if args_from_file.is_empty() {
            // Return the command line arguments, as there is nothing to add to these
            // in this case
            return Ok(commandline);
        }

        // Build combined options from command line arguments and arguments parsed
        // from file by prepending everything from the config file before the
        // command line parameters
        // This way command line params overwrite duplicate options from the config
        // file because they are parsed later
        let mut cliargs = commandline.clone();

        // Shift the first element from the actual command line args to the
        // options that where parsed from the file
        // This is necessary because the first item in the command line arguments
        // is the name of the executable and ignored by clap during parsing
        args_from_file.insert(0, cliargs.remove(0));
        args_from_file.extend(cliargs);

        // Return combined values
        Ok(args_from_file)
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use crate::{ConfigBuilder, ConfigOption, Configurable, Configuration};
    use std::collections::HashMap;
    use std::env;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(1);

    fn get_and_delete_env_var() -> String {
        let name = format!("configfile-{}", COUNTER.fetch_add(1, Ordering::Relaxed));
        env::remove_var(&name);
        name
    }

    // Define a test configuration that can be used to run a few tests
    struct TestConfig {
        values: HashMap<ConfigOption, Option<Vec<String>>>,
    }

    // Test Config object that defines a few very simple config options that can be used for
    // testing the implementation covers all areas
    // Also provides two helper functions for checking the parsing results
    impl TestConfig {
        pub const TEST_PARAM: ConfigOption = ConfigOption {
            name: "testparam",
            default: Some("udtarine"),
            required: false,
            takes_argument: true,
            help: "Testhelp",
            documentation: "Testdoc",
            list: false,
        };
        pub const TEST_PARAM2: ConfigOption = ConfigOption {
            name: "testparam2",
            default: None,
            required: false,
            takes_argument: true,
            help: "test2",
            documentation: "test2",
            list: false,
        };
        pub const TEST_SWITCH: ConfigOption = ConfigOption {
            name: "testswitch",
            default: None,
            required: false,
            takes_argument: false,
            help: "a switch that can be provided - or not",
            documentation: "test doc switch",
            list: false,
        };
        pub const TEST_MULTIPLE: ConfigOption = ConfigOption {
            name: "testmultiple",
            default: Some("3"),
            required: false,
            takes_argument: true,
            help: "A parameter that can be specified multiple times and all values will be used.",
            documentation: "",
            list: true,
        };

        // This function retrieves a string value that is stored for the ConfigOption that
        // is passed to it
        // The function will only return this value if it is the only value that is stored under
        // this ConfigOption, it will panic in any other case:
        //  - no value is stored
        //  - more than one value is stored
        // This allows keeping assert statements a bit briefer in the actual test by removing the
        // need to check for empty or lists larger than one
        pub fn get_first_and_only_value(&self, key: &ConfigOption) -> String {
            let value = self
                .values
                .get(key)
                .expect("Error retrieving value!")
                .clone();
            if value == None {
                panic!("Argument was not specified!");
            }
            let value = value.expect("Shouldn't happen");
            if value.len() != 1 {
                panic!("Not a single value: {}", value.len());
            }
            String::from(&value[0].clone())
        }

        // Helper function to check whether the argument was provided on the command line
        pub fn argument_was_provided(&self, key: &ConfigOption) -> bool {
            if let Some(_v) = self
                .values
                .get(key)
                .expect("Fatal error: key not present in HashMap, but should have been!")
            {
                true
            } else {
                false
            }
        }
    }

    // Implementation to return description of this config that is needed by ConfigBuilder
    impl Configurable for TestConfig {
        fn get_config_description() -> Configuration {
            Configuration {
                name: "Test Tool",
                version: "0.1",
                about: "blabla",
                options: [
                    TestConfig::TEST_PARAM,
                    TestConfig::TEST_PARAM2,
                    TestConfig::TEST_SWITCH,
                    TestConfig::TEST_MULTIPLE,
                ]
                .iter()
                .cloned()
                .collect(),
            }
        }

        // Very simple implementation used for testing purposes only
        // Simply store the HashMap
        fn parse_values(parsed_values: HashMap<ConfigOption, Option<Vec<String>>>) -> Self {
            TestConfig {
                values: parsed_values,
            }
        }
    }

    #[test]
    fn parse_single_param() {
        let env_var_name = get_and_delete_env_var();

        let command_line_args: Vec<OsString> = vec![
            OsString::from("filename"),
            OsString::from("--testparam"),
            OsString::from("param1"),
        ];
        let config: TestConfig =
            ConfigBuilder::build(command_line_args, &env_var_name).expect("test");

        // Check that absent parameters are reported correctly
        assert_eq!(
            config.argument_was_provided(&TestConfig::TEST_SWITCH),
            false
        );
        assert_eq!(
            config.argument_was_provided(&TestConfig::TEST_PARAM2),
            false
        );

        assert_eq!(
            config.get_first_and_only_value(&TestConfig::TEST_PARAM),
            "param1"
        );
    }

    #[test]
    fn parse_multiple_params() {
        let env_var_name = get_and_delete_env_var();
        let command_line_args: Vec<OsString> = vec![
            OsString::from("filename"),
            OsString::from("--testswitch"),
            OsString::from("--testparam"),
            OsString::from("param1"),
            OsString::from("--testparam2"),
            OsString::from("param2"),
        ];
        let config: TestConfig = ConfigBuilder::build(command_line_args, &env_var_name)
            .expect("Error building config object!");

        assert!(config.argument_was_provided(&TestConfig::TEST_SWITCH));

        assert!(config.argument_was_provided(&TestConfig::TEST_PARAM));
        assert_eq!(
            config.get_first_and_only_value(&TestConfig::TEST_PARAM),
            "param1"
        );

        assert!(config.argument_was_provided(&TestConfig::TEST_PARAM2));

        assert_eq!(
            config.get_first_and_only_value(&TestConfig::TEST_PARAM2),
            "param2"
        );

        assert!(config.argument_was_provided(&TestConfig::TEST_SWITCH));
    }

    #[test]
    fn test_parameters_absent() {
        let env_var_name = get_and_delete_env_var();

        let command_line_args: Vec<OsString> = vec![OsString::from("filename")];

        let config: TestConfig = ConfigBuilder::build(command_line_args, &env_var_name)
            .expect("Error building config object!");

        // TestConfig::TestSwitch
        // takes_argument: false
        assert_eq!(
            config.argument_was_provided(&TestConfig::TEST_SWITCH),
            false
        );

        // TestConfig::TestParam
        // takes_argument: true
        // default: "udtarine"
        // list: false
        assert!(config.argument_was_provided(&TestConfig::TEST_PARAM), true);
        assert_eq!(
            config.get_first_and_only_value(&TestConfig::TEST_PARAM),
            TestConfig::TEST_PARAM.default.expect("")
        );

        // TestConfig::TestParam2
        // takes_argument: true
        // no default
        // list: false
        assert_eq!(
            config.argument_was_provided(&TestConfig::TEST_PARAM2),
            false
        );

        // TestConfig::TestMultiple
        // takes_argument: true
        // default: "3"
        // list: true
        assert!(config.argument_was_provided(&TestConfig::TEST_MULTIPLE));
        assert_eq!(
            config.get_first_and_only_value(&TestConfig::TEST_MULTIPLE),
            "3"
        );
    }

    #[test]
    fn parse_from_file_only() {
        let env_var_name = get_and_delete_env_var();

        let command_line_args: Vec<OsString> = vec![OsString::from("filename")];

        env::set_var(
            &env_var_name,
            get_absolute_file("resources/test/config1.conf"),
        );
        let config: TestConfig = ConfigBuilder::build(command_line_args, &env_var_name)
            .expect("Error building config object!");

        assert!(config.argument_was_provided(&TestConfig::TEST_PARAM));

        assert_eq!(
            config.get_first_and_only_value(&TestConfig::TEST_PARAM),
            "fromfile"
        );

        assert!(config.argument_was_provided(&TestConfig::TEST_PARAM2));
    }

    /// This test case specifies the same parameter in a config file and on the command line
    /// Expected result is that command line parameter overrides the file.
    /// To ensure the file is not simply ignored a second parameter is loaded from file only.
    #[test]
    fn override_value_from_file() {
        let env_var_name = get_and_delete_env_var();

        let command_line_args: Vec<OsString> = vec![
            OsString::from("filename"),
            OsString::from("--testparam"),
            OsString::from("param1"),
        ];

        env::set_var(
            &env_var_name,
            get_absolute_file("resources/test/config1.conf"),
        );

        let config: TestConfig = ConfigBuilder::build(command_line_args, &env_var_name)
            .expect("Error building config object!");

        assert!(config.argument_was_provided(&TestConfig::TEST_PARAM));
        assert_eq!(
            config.get_first_and_only_value(&TestConfig::TEST_PARAM),
            "param1"
        );

        assert!(config.argument_was_provided(&TestConfig::TEST_PARAM2));
        assert_eq!(
            config.get_first_and_only_value(&TestConfig::TEST_PARAM2),
            "fromfile2"
        );
    }

    // Test whether multiple occurrences of the same parameter are parsed correctly
    #[test]
    fn test_multiple_values() {
        let env_var_name = get_and_delete_env_var();

        let command_line_args: Vec<OsString> = vec![
            OsString::from("filename"),
            OsString::from("--testmultiple"),
            OsString::from("1"),
            OsString::from("--testmultiple"),
            OsString::from("2"),
            OsString::from("--testmultiple"),
            OsString::from("3"),
        ];
        let config: TestConfig = ConfigBuilder::build(command_line_args, &env_var_name)
            .expect("Error building config object!");
        let result = config
            .values
            .get(&TestConfig::TEST_MULTIPLE)
            .expect("error getting value")
            .clone();
        let result = result.expect("no values specified!");
        assert_eq!(result.len(), 3);
        assert!(result.contains(&String::from("1")));
        assert!(result.contains(&String::from("2")));
        assert!(result.contains(&String::from("3")));
    }

    /// Helper function to convert a filename that is relative to the config crate Cargo.toml
    /// file to an absolute path by retrieving the CARGO_MANIFEST_DIR environment variable
    /// and prepending this to the filename
    ///
    /// * `filename` - A relative filename (no leading /)
    fn get_absolute_file(filename: &str) -> String {
        env!("CARGO_MANIFEST_DIR").to_owned() + "/" + filename
    }
}
