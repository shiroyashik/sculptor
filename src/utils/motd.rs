use chrono::Duration;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::AppState;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Motd {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click_event: Option<ClickEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underlined: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickEvent {
    pub action: String,
    pub value: String,
}

pub async fn get_motd(state: AppState) -> Vec<Motd> {
    let motd_settings = &state.config.read().await.motd;
    
    let custom: Result<Vec<Motd>, serde_json::Error> = serde_json::from_str(&motd_settings.custom_text).map_err(|e| { error!("Can't parse custom MOTD!\n{e:?}"); e});
    if !motd_settings.display_server_info {
        return custom.unwrap();
    }

    // let time = Local::now().format("%H:%M");
    let uptime = state.uptime.elapsed().as_secs();
    let duration = Duration::seconds(uptime.try_into().unwrap());
    let hours = duration.num_hours();
    let minutes = duration.num_minutes() % 60;
    let seconds = duration.num_seconds() % 60;

    let mut ser_info = vec![
        // Motd { 
        //     text: format!("Generated at {time}\n"),
        //     ..Default::default()
        // },
        Motd {
            text: format!("{}{:02}:{:02}:{:02}\n", motd_settings.text_uptime, hours, minutes, seconds),
            ..Default::default()
        },
        Motd { 
            text: format!("{}{}\n", motd_settings.text_authclients, state.user_manager.count_authenticated()),
            ..Default::default()
        },
    ];

    if motd_settings.draw_indent {
        ser_info.push(Motd { 
            text: "----\n\n".to_string(),
            color: Some("gold".to_string()),
            ..Default::default()
        })
    }

    if let Ok(custom) = custom {
        [ser_info, custom].concat()
    } else {
        ser_info
    }
}