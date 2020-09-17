use std::ffi::OsString;
use std::fmt::Error;

use clap::{App, Arg, ArgMatches};

// Include all "stolen" ripgrep code in this module
mod ripgrep_config;

/// This trait defines the behavior that all configuration classes need to
/// provide in order for the clap matcher to be generated from the config object
pub trait ConfigDescription {
    fn get_config(&self) -> Configuration;
}

/// This struct describes some properties that can be set for an application as well
/// as the list of options that the program can understand
/// These values (apart from the list of options) are only used to generate the
/// console help message
pub struct Configuration {
    /// the name of the application
    pub name: &'static str,
    /// version of the application
    pub version: &'static str,
    /// a brief description of what the application does
    pub about: &'static str,
    /// the list of all possible command line options
    pub options: Vec<ConfigOption>,
}

/// Represents an individual config option that the program can interpret
pub struct ConfigOption {
    // the name of the option (without leading --)
    pub name: &'static str,
    // default value to use for the option if it is not provided
    pub default: &'static str,
    // whether this option has to be provided
    pub required: bool,
    // if true the option takes a value as argument, if false
    // the option is a present/missing flag
    pub takes_argument: bool,
    // help text to display for the option
    pub help: &'static str,
    // longer text to use when generating documentation/website/...
    pub documentation: &'static str,
}

/// Function to create a clap matcher from a Configuration struct and use this to parse the
/// command line parameters and return the processed config
///
/// The general flow is like this:
/// 1. Create clap matcher from Configuration
/// 2. Use matcher to parse command line arguments
/// 3. If --no-config parameter was specified return parsed config
/// 4. If --no-config is not present check environment variable STACKABLE_CONFIG_PATH
///    if an external config file is specified
/// 5. Parse config from file and prepend all options to the command line arguments
/// 6. Re-parse combined arguments
/// 7. Return parsed config
///
/// This effectively means that config can be either provided on the command line, or
/// in a file that is specified via environment variable, with options from the command
/// line taking precedence over the config file.
///
/// * `config` The definition of a config that the matcher will be built from
/// * `config_file_env` Name of the environment variable to read an extra config file from
/// * `args` The command line parameters to parse the configuration from (first element will be
/// ignored, as this is the binary name
pub fn get_matcher<'a>(
    config: &dyn ConfigDescription,
    config_file_env: &str,
    args: Vec<OsString>,
) -> Result<ArgMatches<'a>, Error> {
    let configuration = config.get_config();
    let options = &configuration.options;

    let mut matches = App::new(configuration.name)
        .version(configuration.version)
        .about(configuration.about);

    for option in options.iter() {
        matches = matches.arg(
            Arg::with_name(option.name)
                .long(option.name)
                .value_name(option.name)
                .help(option.help)
                .takes_value(option.takes_argument)
                .overrides_with(option.name)
                .required(option.required),
        );
    }

    // Creating a matcher is a fairly expensive operation, so we clone it,
    // in case we need to reuse later for a second pass over the combined
    // arguments
    let new_matcher = matches.clone();

    // Parse provided arguments
    let command_line_args = matches.get_matches_from(args.clone());

    // If --no-config was passed on the command line, we bypass reading values from the
    // extra config file
    let mut args_from_file = if command_line_args.is_present("no-config") {
        vec![]
    } else {
        ripgrep_config::args(config_file_env)
    };

    // Check if there were any arguments in the config file
    if args_from_file.is_empty() {
        // Nothing further to do if there were none
        return Ok(command_line_args);
    }

    // Build combined options from command line arguments and arguments parsed
    // from file by prepending everything from the config file before the
    // command line parameters
    // This way command line params overwrite duplicate options from the config
    // file because teey are parsed later
    let mut cliargs = args;

    // Shift the first element from the actual command line args to the
    // options that where parsed from the file
    // This is necessary because the first item in the command line arguments
    // is the name of the executable and ignored by clap during parsing
    args_from_file.insert(0, cliargs.remove(0));
    args_from_file.extend(cliargs);
    // TODO: Convert to debug log statement
    println!("final argv: {:?}", args_from_file);

    // Return parsed config
    Ok(new_matcher.get_matches_from(args_from_file))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use crate::{get_matcher, ConfigDescription, ConfigOption, Configuration};
    use std::env;

    // Define a test configuration that can be used to run a few tests
    struct TestConfig {}

    impl TestConfig {
        pub const TEST_PARAM: ConfigOption = ConfigOption {
            name: "testparam",
            default: "",
            required: false,
            takes_argument: true,
            help: "Testhelp",
            documentation: "Testdoc",
        };
        pub const TEST_PARAM2: ConfigOption = ConfigOption {
            name: "testparam2",
            default: "2",
            required: false,
            takes_argument: true,
            help: "test2",
            documentation: "test2",
        };
        pub const TEST_SWITCH: ConfigOption = ConfigOption {
            name: "testswitch",
            default: "",
            required: false,
            takes_argument: false,
            help: "a switch that can be provided - or not",
            documentation: "test doc switch",
        };
    }

    impl ConfigDescription for TestConfig {
        fn get_config(&self) -> Configuration {
            Configuration {
                name: "Test Tool",
                version: "0.1",
                about: "blabla",
                options: vec![
                    TestConfig::TEST_PARAM,
                    TestConfig::TEST_PARAM2,
                    TestConfig::TEST_SWITCH,
                ],
            }
        }
    }

    #[test]
    fn parse_single_param() {
        let config = TestConfig {};
        let command_line_args: Vec<OsString> = vec![
            OsString::from("filename"),
            OsString::from("--testparam"),
            OsString::from("param1"),
        ];
        let matcher = get_matcher(&config, &"Test", command_line_args)
            .expect("unexpected error occurred when parsing parameters");

        assert!(matcher.is_present(TestConfig::TEST_PARAM.name));
        assert_eq!(
            matcher.value_of(TestConfig::TEST_PARAM.name).unwrap(),
            "param1"
        );
    }

    #[test]
    fn parse_multiple_params() {
        let config = TestConfig {};
        let command_line_args: Vec<OsString> = vec![
            OsString::from("filename"),
            OsString::from("--testswitch"),
            OsString::from("--testparam"),
            OsString::from("param1"),
            OsString::from("--testparam2"),
            OsString::from("param2"),
        ];
        let matcher = get_matcher(&config, &"Test", command_line_args)
            .expect("unexpected error occurred when parsing parameters");

        assert!(matcher.is_present(TestConfig::TEST_PARAM.name));
        assert_eq!(
            matcher.value_of(TestConfig::TEST_PARAM.name).unwrap(),
            "param1"
        );

        assert!(matcher.is_present(TestConfig::TEST_PARAM2.name));
        assert_eq!(
            matcher.value_of(TestConfig::TEST_PARAM2.name).unwrap(),
            "param2"
        );

        assert!(matcher.is_present(TestConfig::TEST_SWITCH.name));
    }

    #[test]
    fn parse_from_file_only() {
        let config = TestConfig {};
        let command_line_args: Vec<OsString> = vec![OsString::from("filename")];

        env::set_var(
            "CONFIG_FILE",
            get_absolute_file("resources/test/config1.conf"),
        );
        let matcher = get_matcher(&config, &"CONFIG_FILE", command_line_args)
            .expect("unexpected error occurred when parsing parameters");

        assert!(matcher.is_present(TestConfig::TEST_PARAM.name));
        assert_eq!(
            matcher.value_of(TestConfig::TEST_PARAM.name).unwrap(),
            "fromfile"
        );
        assert!(matcher.is_present(TestConfig::TEST_PARAM2.name));
        assert_eq!(
            matcher.value_of(TestConfig::TEST_PARAM2.name).unwrap(),
            "fromfile2"
        );
    }

    /// This test case specifies the same parameter in a config file and on the command line
    /// Expected result is that command line parameter overrides the file.
    /// To ensure the file is not simply ignored a second parameter is loaded from file only.
    #[test]
    fn override_value_from_file() {
        let config = TestConfig {};
        let command_line_args: Vec<OsString> = vec![
            OsString::from("filename"),
            OsString::from("--testparam"),
            OsString::from("param1"),
        ];

        env::set_var(
            "CONFIG_FILE",
            get_absolute_file("resources/test/config1.conf"),
        );
        let matcher = get_matcher(&config, &"CONFIG_FILE", command_line_args)
            .expect("unexpected error occurred when parsing parameters");

        assert!(matcher.is_present(TestConfig::TEST_PARAM.name));
        assert_eq!(
            matcher.value_of(TestConfig::TEST_PARAM.name).unwrap(),
            "param1"
        );
        assert!(matcher.is_present(TestConfig::TEST_PARAM2.name));
        assert_eq!(
            matcher.value_of(TestConfig::TEST_PARAM2.name).unwrap(),
            "fromfile2"
        );
    }

    /// Convert a filename that is relative to the config crate Cargo.toml file
    /// to an absolute path by retrieving the CARGO_MANIFEST_DIR environment variable
    /// and prepending this to the filename
    ///
    /// * `filename` - A relative filename (no leading /)
    fn get_absolute_file(filename: &str) -> String {
        env!("CARGO_MANIFEST_DIR").to_owned() + &"/" + filename
    }
}
