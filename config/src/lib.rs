use clap::{App, Arg, ArgMatches};
use std::env;
use std::fmt::Error;

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
    /// the name of an environment variable that can be used to specify an
    /// additional config file
    pub environment_config: &'static str,
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
pub fn get_matcher<'a>(config: &dyn ConfigDescription) -> Result<ArgMatches<'a>, Error> {
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

    // Parse command line arguments
    let command_line_args = matches.get_matches();

    // If --no-config was passed on the command line, we bypass reading values from the
    // extra config file
    let mut args_from_file = if command_line_args.is_present("no-config") {
        vec![]
    } else {
        ripgrep_config::args(configuration.environment_config)
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
    // file because they are parsed later
    let mut cliargs = env::args_os();
    if let Some(bin) = cliargs.next() {
        args_from_file.insert(0, bin);
    }
    args_from_file.extend(cliargs);
    // TODO: Convert to debug log statement
    println!("final argv: {:?}", args_from_file);

    // Return parsed config
    Ok(new_matcher.get_matches_from(args_from_file))
}