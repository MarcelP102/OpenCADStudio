use crate::modules::{IconKind, ModuleEvent, ToolDef};

pub fn tool() -> ToolDef {
    ToolDef {
        id: "LAYLCK",
        label: "Layer Lock",
        icon: IconKind::Svg(include_bytes!("../../../../assets/icons/v2/lock.svg")),
        event: ModuleEvent::Command("LAYLCK".to_string()),
    }
}
