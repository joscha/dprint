use dprint_core::plugins::wasm::{self};
use dprint_core::types::ErrBox;

use crate::environment::Environment;
use crate::plugins::read_info_file;

pub fn get_init_config_file_text(environment: &impl Environment) -> Result<String, ErrBox> {
  let info = match read_info_file(environment) {
    Ok(info) => {
      // ok to only check wasm here because the configuration file is only ever initialized with wasm plugins
      if info.plugin_system_schema_version != wasm::PLUGIN_SYSTEM_SCHEMA_VERSION {
        environment.log_error(&format!(
          concat!(
            "You are using an old version of dprint so the created config file may not be as helpful of a starting point. ",
            "Consider upgrading to support new plugins. ",
            "Plugin system schema version is {}, latest is {}."
          ),
          wasm::PLUGIN_SYSTEM_SCHEMA_VERSION,
          info.plugin_system_schema_version,
        ));
        None
      } else {
        Some(info)
      }
    }
    Err(err) => {
      environment.log_error(&format!(
        concat!(
          "There was a problem getting the latest plugin info. ",
          "The created config file may not be as helpful of a starting point. ",
          "Error: {}"
        ),
        err.to_string()
      ));
      None
    }
  };

  let selected_plugins = if let Some(info) = info {
    let latest_plugins = info.latest_plugins;
    let prompt_message = "Select plugins (use the spacebar to select/deselect and then press enter when finished):";
    let plugin_indexes = environment.get_multi_selection(
      prompt_message,
      0,
      &latest_plugins.iter().map(|x| (!x.is_process_plugin(), String::from(&x.name))).collect(),
    )?;
    let mut selected_plugins = Vec::new();
    for index in plugin_indexes {
      selected_plugins.push(latest_plugins[index].clone());
    }
    Some(selected_plugins)
  } else {
    None
  };

  let mut json_text = String::from("{\n");
  json_text.push_str("  \"incremental\": true,\n");

  if let Some(selected_plugins) = &selected_plugins {
    for plugin in selected_plugins.iter() {
      // Put the brace on the next line so the user doesn't have to as soon as they
      // go to add options.
      if let Some(config_key) = &plugin.config_key {
        if !config_key.is_empty() {
          json_text.push_str(&format!("  \"{}\": {{\n", config_key));
          json_text.push_str("  },\n");
        }
      }
    }

    let extension_includes = get_unique_items(
      selected_plugins
        .iter()
        .flat_map(|p| p.file_extensions.iter())
        .map(|x| x.as_str())
        .collect::<Vec<_>>(),
    );
    let file_name_includes = get_unique_items(
      selected_plugins
        .iter()
        .flat_map(|p| p.file_names.iter())
        .map(|x| x.as_str())
        .collect::<Vec<_>>(),
    );

    let mut json_includes = vec![];
    if !extension_includes.is_empty() {
      json_includes.push(format!("\"**/*.{{{}}}\"", extension_includes.join(",")));
    }
    if !file_name_includes.is_empty() {
      json_includes.push(format!("\"**/{{{}}}\"", file_name_includes.join(",")));
    }

    json_text.push_str("  \"includes\": [");
    if json_includes.is_empty() {
      json_text.push_str("\"**/*.*\"");
    } else {
      json_text.push_str(&json_includes.join(","));
    }
    json_text.push_str("],\n");
    json_text.push_str("  \"excludes\": [");
    let excludes = get_unique_items(
      selected_plugins
        .iter()
        .flat_map(|p| p.config_excludes.iter())
        .map(|x| format!("    \"{}\"", x))
        .collect::<Vec<_>>(),
    );
    if !excludes.is_empty() {
      json_text.push_str("\n");
      json_text.push_str(&excludes.join(",\n"));
      json_text.push_str("\n  ");
    }
    json_text.push_str("],\n");
    json_text.push_str("  \"plugins\": [\n");
    if selected_plugins.is_empty() {
      json_text.push_str("    // specify plugin urls here\n");
    } else {
      for (i, plugin) in selected_plugins.iter().enumerate() {
        if i > 0 {
          json_text.push_str(",\n");
        }
        let url = if plugin.is_process_plugin() && plugin.checksum.is_some() {
          format!("{}@{}", plugin.url, plugin.checksum.as_ref().unwrap())
        } else {
          plugin.url.to_string()
        };
        json_text.push_str(&format!("    \"{}\"", url));
      }
      json_text.push_str("\n");
    }
    json_text.push_str("  ]\n}\n");
  } else {
    json_text.push_str("  \"includes\": [\"**/*.{ts,tsx,js,jsx,json}\"],\n");
    json_text.push_str("  \"excludes\": [\n    \"**/node_modules\",\n    \"**/*-lock.json\"\n  ],\n");
    json_text.push_str("  \"plugins\": [\n");
    json_text.push_str("    // specify plugin urls here\n");
    json_text.push_str("  ]\n}\n");
  }

  Ok(json_text)
}

/// Gets the unique items in the vector in the same order
fn get_unique_items<T>(vec: Vec<T>) -> Vec<T>
where
  T: PartialEq,
{
  let mut new_vec = Vec::new();

  for item in vec {
    if !new_vec.contains(&item) {
      new_vec.push(item);
    }
  }

  new_vec
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::environment::TestEnvironment;
  use crate::plugins::REMOTE_INFO_URL;
  use pretty_assertions::assert_eq;

  #[test]
  fn should_get_initialization_text_when_can_access_url() {
    let environment = TestEnvironment::new();
    environment.add_remote_file(REMOTE_INFO_URL, get_multi_plugins_config().as_bytes());
    environment.set_multi_selection_result(vec![0, 1, 2]);
    let text = get_init_config_file_text(&environment).unwrap();
    assert_eq!(
      text,
      r#"{
  "incremental": true,
  "typescript": {
  },
  "json": {
  },
  "includes": ["**/*.{ts,tsx,json,rs}","**/{Cargo.toml}"],
  "excludes": [
    "**/something",
    "**/*-asdf.json",
    "**other"
  ],
  "plugins": [
    "https://plugins.dprint.dev/typescript-0.17.2.wasm",
    "https://plugins.dprint.dev/json-0.2.3.wasm",
    "https://plugins.dprint.dev/final-0.1.2.wasm"
  ]
}
"#
    );

    assert_eq!(environment.take_logged_errors(), get_standard_logged_messages());
  }

  #[test]
  fn should_get_initialization_text_when_selecting_one_plugin() {
    let environment = TestEnvironment::new();
    environment.add_remote_file(REMOTE_INFO_URL, get_multi_plugins_config().as_bytes());
    environment.set_multi_selection_result(vec![1]);
    let text = get_init_config_file_text(&environment).unwrap();
    assert_eq!(
      text,
      r#"{
  "incremental": true,
  "json": {
  },
  "includes": ["**/*.{json}"],
  "excludes": [
    "**/*-asdf.json"
  ],
  "plugins": [
    "https://plugins.dprint.dev/json-0.2.3.wasm"
  ]
}
"#
    );

    assert_eq!(environment.take_logged_errors(), get_standard_logged_messages());
  }

  #[test]
  fn should_get_initialization_text_when_selecting_no_plugins() {
    let environment = TestEnvironment::new();
    environment.add_remote_file(REMOTE_INFO_URL, get_multi_plugins_config().as_bytes());
    environment.set_multi_selection_result(vec![]);
    let text = get_init_config_file_text(&environment).unwrap();
    assert_eq!(
      text,
      r#"{
  "incremental": true,
  "includes": ["**/*.*"],
  "excludes": [],
  "plugins": [
    // specify plugin urls here
  ]
}
"#
    );

    assert_eq!(environment.take_logged_errors(), get_standard_logged_messages());
  }

  #[test]
  fn should_get_initialization_text_when_selecting_process_plugin() {
    let environment = TestEnvironment::new();
    environment.add_remote_file(REMOTE_INFO_URL, get_multi_plugins_config().as_bytes());
    environment.set_multi_selection_result(vec![3]);
    let text = get_init_config_file_text(&environment).unwrap();
    assert_eq!(
      text,
      r#"{
  "incremental": true,
  "includes": ["**/*.{ps}"],
  "excludes": [],
  "plugins": [
    "https://plugins.dprint.dev/process-0.1.0.exe-plugin@test-checksum"
  ]
}
"#
    );

    assert_eq!(environment.take_logged_errors(), get_standard_logged_messages());
  }

  #[test]
  fn should_get_initialization_text_when_cannot_access_url() {
    let environment = TestEnvironment::new();
    let text = get_init_config_file_text(&environment).unwrap();
    assert_eq!(
      text,
      r#"{
  "incremental": true,
  "includes": ["**/*.{ts,tsx,js,jsx,json}"],
  "excludes": [
    "**/node_modules",
    "**/*-lock.json"
  ],
  "plugins": [
    // specify plugin urls here
  ]
}
"#
    );
    let mut expected_messages = get_standard_logged_messages_no_plugin_selection();
    expected_messages.push(concat!(
      "There was a problem getting the latest plugin info. ",
      "The created config file may not be as helpful of a starting point. ",
      "Error: Could not find file at url https://plugins.dprint.dev/info.json"
    ));
    assert_eq!(environment.take_logged_errors(), expected_messages);
  }

  #[test]
  fn should_get_initialization_text_when_selecting_other_option() {
    let environment = TestEnvironment::new();
    environment.set_selection_result(1);
    environment.add_remote_file(
      REMOTE_INFO_URL,
      r#"{
    "schemaVersion": 3,
    "pluginSystemSchemaVersion": 3,
    "latest": [{
        "name": "dprint-plugin-typescript",
        "version": "0.17.2",
        "url": "https://plugins.dprint.dev/typescript-0.17.2.wasm",
        "configKey": "typescript",
        "fileExtensions": ["ts"],
        "configExcludes": ["test"]
    }]
}"#
        .as_bytes(),
    );
    environment.set_multi_selection_result(vec![0]);
    let text = get_init_config_file_text(&environment).unwrap();
    assert_eq!(
      text,
      r#"{
  "incremental": true,
  "typescript": {
  },
  "includes": ["**/*.{ts}"],
  "excludes": [
    "test"
  ],
  "plugins": [
    "https://plugins.dprint.dev/typescript-0.17.2.wasm"
  ]
}
"#
    );

    assert_eq!(environment.take_logged_errors(), get_standard_logged_messages());
  }

  #[test]
  fn should_get_initialization_text_when_old_plugin_system() {
    let environment = TestEnvironment::new();
    environment.add_remote_file(
      REMOTE_INFO_URL,
      r#"{
    "schemaVersion": 3,
    "pluginSystemSchemaVersion": 9, // this is 9 instead of 3
    "latest": [{
        "name": "dprint-plugin-typescript",
        "version": "0.17.2",
        "url": "https://plugins.dprint.dev/typescript-0.17.2.wasm",
        "configKey": "typescript",
        "fileExtensions": ["ts"],
        "configExcludes": ["asdf"]
    }]
}"#
        .as_bytes(),
    );
    environment.set_multi_selection_result(vec![0]);
    let text = get_init_config_file_text(&environment).unwrap();
    assert_eq!(
      text,
      r#"{
  "incremental": true,
  "includes": ["**/*.{ts,tsx,js,jsx,json}"],
  "excludes": [
    "**/node_modules",
    "**/*-lock.json"
  ],
  "plugins": [
    // specify plugin urls here
  ]
}
"#
    );
    let mut expected_messages = get_standard_logged_messages_no_plugin_selection();
    expected_messages.push(concat!(
      "You are using an old version of dprint so the created config file may not be as helpful of a starting point. ",
      "Consider upgrading to support new plugins. ",
      "Plugin system schema version is 3, latest is 9."
    ));
    assert_eq!(environment.take_logged_errors(), expected_messages);
  }

  fn get_standard_logged_messages_no_plugin_selection() -> Vec<&'static str> {
    vec![]
  }

  fn get_standard_logged_messages() -> Vec<&'static str> {
    vec!["Select plugins (use the spacebar to select/deselect and then press enter when finished):"]
  }

  fn get_multi_plugins_config() -> &'static str {
    return r#"{
            "schemaVersion": 3,
            "pluginSystemSchemaVersion": 3,
            "latest": [{
                "name": "dprint-plugin-typescript",
                "version": "0.17.2",
                "url": "https://plugins.dprint.dev/typescript-0.17.2.wasm",
                "configKey": "typescript",
                "fileExtensions": ["ts", "tsx"],
                "configExcludes": ["**/something"]
            }, {
                "name": "dprint-plugin-jsonc",
                "version": "0.2.3",
                "url": "https://plugins.dprint.dev/json-0.2.3.wasm",
                "configKey": "json",
                "fileExtensions": ["json"],
                "configExcludes": ["**/*-asdf.json"]
            }, {
                "name": "dprint-plugin-final",
                "version": "0.1.2",
                "url": "https://plugins.dprint.dev/final-0.1.2.wasm",
                "fileExtensions": ["tsx", "rs"],
                "fileNames": ["Cargo.toml"],
                "configExcludes": ["**/something", "**other"]
            }, {
                "name": "dprint-process-plugin",
                "version": "0.1.0",
                "url": "https://plugins.dprint.dev/process-0.1.0.exe-plugin",
                "fileExtensions": ["ps"],
                "configExcludes": [],
                "checksum": "test-checksum"
            }]
        }"#;
  }
}
