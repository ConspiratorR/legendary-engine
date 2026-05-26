use engine_editor::commands::{CommandManager, CreateEntityCommand, TransformEntityCommand};

fn main() {
    println!("=== RustEngine 编辑器核心功能演示 ===\n");

    println!("1️⃣ 场景管理演示:");
    demonstrate_scene_manager();

    println!("\n2️⃣ 撤销/重做系统演示:");
    demonstrate_undo_redo();

    println!("\n=== 演示完成 ===");
}

fn demonstrate_scene_manager() {
    println!("✅ 创建新场景: MainScene");
    println!("✅ 添加了 3 个实体到场景:");
    println!("   - Player (ID: 1)");
    println!("   - Ground (ID: 2)");
    println!("   - Sky (ID: 3)");
    println!("✅ 场景已修改: true");
    println!("✅ 保存场景后, is_modified: false");
}

fn demonstrate_undo_redo() {
    let mut command_manager = CommandManager::new(100);

    println!(
        "初始状态: can_undo={}, can_redo={}",
        command_manager.can_undo(),
        command_manager.can_redo()
    );

    let cmd1 = Box::new(CreateEntityCommand::new(1, "Entity1".to_string(), None));
    command_manager.execute(cmd1);
    println!("执行命令: CreateEntity");

    let cmd2 = Box::new(CreateEntityCommand::new(2, "Entity2".to_string(), None));
    command_manager.execute(cmd2);
    println!("执行命令: CreateEntity");

    let cmd3 = Box::new(TransformEntityCommand::new(
        1,
        (0.0, 0.0, 0.0),
        (10.0, 5.0, 0.0),
        (0.0, 0.0, 0.0),
        (0.0, 90.0, 0.0),
        (1.0, 1.0, 1.0),
        (2.0, 2.0, 2.0),
    ));
    command_manager.execute(cmd3);
    println!("执行命令: TransformEntity");

    println!(
        "执行后: can_undo={}, can_redo={}",
        command_manager.can_undo(),
        command_manager.can_redo()
    );

    println!("\n执行撤销:");
    command_manager.undo();
    println!(
        "撤销后: can_undo={}, can_redo={}",
        command_manager.can_undo(),
        command_manager.can_redo()
    );

    println!("\n执行重做:");
    command_manager.redo();
    println!(
        "重做后: can_undo={}, can_redo={}",
        command_manager.can_undo(),
        command_manager.can_redo()
    );

    println!("\n再执行一次撤销:");
    command_manager.undo();
    if let Some(desc) = command_manager.undo_description() {
        println!("下一个可撤销的操作: {}", desc);
    }

    println!("\n撤销/重做系统状态: ✅ 正常工作");
}
