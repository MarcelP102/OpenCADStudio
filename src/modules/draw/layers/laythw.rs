use crate::modules::{IconKind, ModuleEvent, ToolDef};

pub fn tool() -> ToolDef {
    ToolDef {
        id: "LAYTHW",
        label: "Thaw All Layers",
        icon: IconKind::Svg(include_bytes!("../../../../assets/icons/v2/freeze.svg")),
        event: ModuleEvent::Command("LAYTHW".to_string()),
    }
}
