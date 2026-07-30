use toml_edit::DocumentMut;

use super::types::{
    EasytierConfig, EasytierConfigMode, ExtensionId, ExtensionsConfig, FrpProxyType, FrpcConfig,
    FrpcConfigMode, GostcConfig,
};

pub(crate) fn validate_extension_config(
    id: ExtensionId,
    config: &ExtensionsConfig,
) -> Result<(), String> {
    match id {
        ExtensionId::Ttyd => Ok(()),
        ExtensionId::Gostc => validate_gostc_config(&config.gostc),
        ExtensionId::Easytier => validate_easytier_config(&config.easytier),
        ExtensionId::Frpc => validate_frpc_config(&config.frpc),
    }
}

pub(crate) fn validate_gostc_config(config: &GostcConfig) -> Result<(), String> {
    require_non_empty(config.addr.trim(), "GOSTC server address is required")?;
    require_non_empty(config.key.as_str(), "GOSTC client key is required")
}

pub(crate) fn validate_easytier_config(config: &EasytierConfig) -> Result<(), String> {
    match config.config_mode {
        EasytierConfigMode::Quick => require_non_empty(
            config.network_name.trim(),
            "EasyTier network name is required",
        ),
        EasytierConfigMode::Full => validate_full_toml("EasyTier", config.custom_toml.as_str()),
    }
}

pub(crate) fn validate_frpc_config(config: &FrpcConfig) -> Result<(), String> {
    match config.config_mode {
        FrpcConfigMode::Quick => {
            require_non_empty(config.proxy_name.trim(), "FRPC proxy name is required")?;
            require_non_empty(config.server_addr.trim(), "FRPC server address is required")?;
            require_non_empty(config.token.as_str(), "FRPC token is required")?;
            require_non_empty(config.local_ip.trim(), "FRPC local IP is required")?;

            if matches!(config.proxy_type, FrpProxyType::Tcp | FrpProxyType::Udp)
                && config.remote_port.is_none()
            {
                return Err("FRPC remote port is required for TCP/UDP proxies".to_string());
            }

            Ok(())
        }
        FrpcConfigMode::Full => validate_full_toml("FRPC", config.custom_toml.as_str()),
    }
}

fn require_non_empty(value: &str, message: &str) -> Result<(), String> {
    if value.is_empty() {
        Err(message.to_string())
    } else {
        Ok(())
    }
}

fn validate_full_toml(extension_name: &str, config: &str) -> Result<(), String> {
    let trimmed = config.trim();
    if trimmed.is_empty() {
        return Err(format!("{} full configuration is required", extension_name));
    }

    trimmed.parse::<DocumentMut>().map_err(|error| {
        format!(
            "{} full configuration is not valid TOML: {}",
            extension_name, error
        )
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_easytier_full_configuration() {
        let mut config = EasytierConfig {
            config_mode: EasytierConfigMode::Full,
            ..Default::default()
        };

        assert_eq!(
            validate_easytier_config(&config).unwrap_err(),
            "EasyTier full configuration is required"
        );

        config.custom_toml = "instance_name = [".to_string();
        assert!(validate_easytier_config(&config)
            .unwrap_err()
            .starts_with("EasyTier full configuration is not valid TOML:"));

        config.custom_toml = "instance_name = \"one-kvm\"".to_string();
        assert!(validate_easytier_config(&config).is_ok());
    }

    #[test]
    fn validates_frpc_through_the_same_entry_point() {
        let mut config = FrpcConfig {
            config_mode: FrpcConfigMode::Full,
            ..Default::default()
        };

        assert_eq!(
            validate_frpc_config(&config).unwrap_err(),
            "FRPC full configuration is required"
        );

        config.custom_toml = "serverAddr = \"frps.example.com\"".to_string();
        assert!(validate_frpc_config(&config).is_ok());
    }
}
