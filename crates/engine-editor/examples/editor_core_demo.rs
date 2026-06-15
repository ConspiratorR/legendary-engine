use engine_editor::commands::{CommandManager, TransformEntityCommand};
use engine_editor::state::EditorState;

fn main() {
    println!("=== RustEngine 编辑器核心功能演示 ===\n");

    println!("1️⃣ 场景管理演示:");
    demonstrate_scene_manager();

    println!("\n2️⃣ 撤销/重做系统演示:");
    demonstrate_undo_redo();

    println!("\n=== 演示完成 ===");
}

fn demonstrate_scene_manager() {
    let state = EditorState::new();
    println!("✅ 创建新场景: 包含 {} 个节点", state.scene_tree.nodes.len());
    println!("✅ 场景已修改: {}", state.scene_manager.is_modified());
}

fn demonstrate_undo_redo() {
    let mut state = EditorState::new();
    let mut command_manager = CommandManager::new(100);

    println!(
        "初始状态: can_undo={}, can_redo={}",
        command_manager.can_undo(),
        command_manager.can_redo()
    );

    let old_transform = [0.0; 9];
    let new_transform = [10.0, 5.0, 0.0, 0.0, 90.0, 0.0, 2.0, 2.0, 2.0];
    let cmd = Box::new(TransformEntityCommand::new(1, old_transform, new_transform));
    command_manager.execute(cmd, &mut state);
    println!("执行命令: TransformEntity");

    println!(
        "执行后: can_undo={}, can_redo={}",
        command_manager.can_undo(),
        command_manager.can_redo()
    );

    println!("\n执行撤销:");
    command_manager.undo(&mut state);
    println!(
        "撤销后: can_undo={}, can_redo={}",
        command_manager.can_undo(),
        command_manager.can_redo()
    );

    println!("\n执行重做:");
    command_manager.redo(&mut state);
    println!(
        "重做后: can_undo={}, can_redo={}",
        command_manager.can_undo(),
        command_manager.can_redo()
    );

    println!("\n再执行一次撤销:");
    command_manager.undo(&mut state);
    if let Some(desc) = command_manager.undo_description() {
        println!("下一个可撤销的操作: {}", desc);
    }

    println!("\n撤销/重做系统状态: ✅ 正常工作");
}
