use crate::modules::{IconKind, ModuleEvent, ToolDef};

pub fn tool() -> ToolDef {
    ToolDef {
        id: "COPYCLIP",
        label: "Copy",
        icon: IconKind::Svg(include_bytes!("../../../../assets/icons/v2/cclip.svg")),
        event: ModuleEvent::Command("COPYCLIP".to_string()),
    }
}
