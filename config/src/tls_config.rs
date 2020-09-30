use crate::ConfigDescription;
use crate::ConfigOption;
use crate::Configuration;

/// The settings defined in this struct are common to all components that employ SSL
/// for transport layer security and potentially also authentication.
/// Not all settings are always needed, in a scenario without client authentication
/// no keystore is necessary for example.

pub struct TlsConfig {}

impl TlsConfig {
    /// A setting to provide the path to a file which will be used as keystore
    pub const KEYSTORE_LOCATION: ConfigOption = ConfigOption {
        name: "tls-keystore-location",
        default: "",
        required: false,
        takes_argument: true,
        help: "The location of the keystore to use when connecting to the orchestrator, keystore \
        should be in PKCS12 format.",
        documentation: "Specify a file in PKCS12 format that should be used to obtain keys\
        used for encryption.\
        The keystore can contain additional keys beside the needed one, in that case the first \
        suitable key that is found will be used.",
    };

    /// A setting to provide the password to be used to open the keystore that
    /// was provided
    pub const KEYSTORE_PASSWORD: ConfigOption = ConfigOption {
        name: "tls-keystore-password",
        default: "",
        required: false,
        takes_argument: true,
        help: "The password that is necessary to access the keystore, if one is required.",
        documentation: "The password that is necessary to access the keystore, if one is required.",
    };

    pub const TRUSTSTORE_LOCATION: ConfigOption = ConfigOption {
        name: "tls-truststore-location",
        default: "",
        required: false,
        takes_argument: true,
        help: "The location of the truststore to use when connecting to the orchestrator.",
        documentation:
            "Specify a file in PKCS12 format that should be used to check if certificates\
        are signed by a trusted authority. \
        Any certificate that was signed with the private key belonging to one of the public keys\
        in this truststore will be accepted as a valid certificte by this client.",
    };

    pub const TRUSTSTORE_PASSWORD: ConfigOption = ConfigOption {
        name: "tls-truststore-password",
        default: "",
        required: false,
        takes_argument: true,
        help: "The password that is necessary to access the truststore, if one is required.",
        documentation:
            "The password that is necessary to access the truststore, if one is required.",
    };

    // TODO: Define sensible defaults
    pub const ENABLED_CIPHERS: ConfigOption = ConfigOption {
        name: "tls-enabled-ciphers",
        default: "",
        required: false,
        takes_argument: true,
        help: "Cipher suites that are accepted when negotiating an encryption mode.",
        documentation: "This parameter allows whitelisting the cipher suites that are acceptable \
        when initiating a secured connection.\
        If left blank the default list of supported ciphers provided by rust-tls will be used.\
        For a list of possible values please refer to https://docs.rs/rustls/0.18.1/rustls/enum.CipherSuite.html",
    };

    // TODO: Define sensible defaults
    pub const ENABLED_PROTOCOLS: ConfigOption = ConfigOption {
        name: "tls-enabled-protocols",
        default: "",
        required: false,
        takes_argument: true,
        help: "A list of acceptable protocol versions to use.",
        documentation: "This defines the protocol versions that may be used. Any client trying to \
        connect or server that we are trying to connect to which does not support one of the versions\
        listed here will be rejected and no connection be possible.",
    };

    fn get_options() -> Vec<ConfigOption> {
        vec![
            TlsConfig::KEYSTORE_LOCATION,
            TlsConfig::KEYSTORE_PASSWORD,
            TlsConfig::TRUSTSTORE_LOCATION,
            TlsConfig::TRUSTSTORE_PASSWORD,
            TlsConfig::ENABLED_CIPHERS,
            TlsConfig::ENABLED_PROTOCOLS,
        ]
    }
}

impl ConfigDescription for TlsConfig {
    fn get_config(&self) -> Configuration {
        Configuration {
            name: "Stackable-TLS Options",
            version: "0.1",
            about:
                "Not intended for direct use in a command line tool, library of TLS options to be\
            added to other config.",
            options: TlsConfig::get_options(),
        }
    }
}
