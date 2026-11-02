use crate::modules::{IconKind, ModuleEvent, ToolDef};
pub fn tool() -> ToolDef {
    ToolDef {
        id: "OPTIONS",
        label: "Options",
        icon: IconKind::Svg(include_bytes!("../../../assets/icons/v2/settings.svg")),
        event: ModuleEvent::Command("OPTIONS".to_string()),
    }
}
