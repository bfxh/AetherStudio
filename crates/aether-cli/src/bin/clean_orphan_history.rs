//! 历史会话记录清理工具
//!
//! 直接操作 SQLite 数据库，支持两种模式：
//! - 默认：清理无工作区绑定的历史会话（workspace_hash 为空）
//! - --all：清空全部历史会话记录

use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let clear_all = args.iter().any(|a| a == "--all");

    let db_path = dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Aether")
        .join("conversations")
        .join("aether_memory.db");

    if !db_path.exists() {
        eprintln!("数据库不存在: {}", db_path.display());
        std::process::exit(1);
    }

    println!("数据库路径: {}", db_path.display());

    let conn = match rusqlite::Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("打开数据库失败: {}", e);
            std::process::exit(1);
        }
    };

    // 查询总记录数
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM conversations", [], |r| r.get(0))
        .unwrap_or(0);
    println!("总会话数: {}", total);

    // 显示所有记录的 workspace_hash 分布
    {
        let mut stmt = conn
            .prepare("SELECT workspace_hash, COUNT(*) as cnt FROM conversations GROUP BY workspace_hash ORDER BY cnt DESC")
            .expect("准备查询失败");
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .expect("查询失败");

        println!("\nworkspace_hash 分布:");
        for row in rows {
            if let Ok((hash, count)) = row {
                let display = if hash.is_empty() {
                    "(空)".to_string()
                } else {
                    hash
                };
                println!("  {}: {} 条", display, count);
            }
        }
    }

    // 查询消息总数
    let msg_total: i64 = conn
        .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))
        .unwrap_or(0);
    println!("消息总数: {}", msg_total);

    if clear_all {
        // ===== 清空全部历史记录 =====
        if total == 0 {
            println!("\n数据库已为空，无需清理。");
            return;
        }

        // 显示将要删除的记录
        {
            let mut stmt = conn
                .prepare("SELECT id, title, mode, message_count FROM conversations ORDER BY updated_at DESC")
                .expect("准备查询失败");
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })
                .expect("查询失败");

            println!("\n将要删除的全部记录:");
            for row in rows {
                if let Ok((id, title, mode, count)) = row {
                    println!("  - [{}] {} ({} 条消息, 模式: {})", id, title, count, mode);
                }
            }
        }

        // 确认删除
        print!(
            "\n确认清空全部 {} 条会话（{} 条消息）吗？此操作不可恢复！(y/N): ",
            total, msg_total
        );
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("已取消。");
            return;
        }

        // 执行级联删除
        let tx = match conn.unchecked_transaction() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("开启事务失败: {}", e);
                std::process::exit(1);
            }
        };

        // 1. 删除全部向量索引
        let vec_deleted = tx.execute("DELETE FROM vec_messages", []).unwrap_or(0);
        println!("已删除 {} 条向量索引", vec_deleted);

        // 2. 删除全部消息
        let msg_deleted = tx.execute("DELETE FROM messages", []).unwrap_or(0);
        println!("已删除 {} 条消息", msg_deleted);

        // 3. 删除全部会话
        let conv_deleted = tx.execute("DELETE FROM conversations", []).unwrap_or(0);
        println!("已删除 {} 条会话", conv_deleted);

        if let Err(e) = tx.commit() {
            eprintln!("提交事务失败: {}", e);
            std::process::exit(1);
        }

        println!(
            "\n清空完成！共删除 {} 条会话、{} 条消息。",
            conv_deleted, msg_deleted
        );
    } else {
        // ===== 清理无工作区绑定的记录 =====
        let orphan_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM conversations WHERE workspace_hash = '' OR workspace_hash IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        println!("无工作区绑定的会话数: {}", orphan_count);

        if orphan_count == 0 {
            println!("没有需要清理的记录。");
            return;
        }

        // 显示将要删除的记录
        {
            let mut stmt = conn
                .prepare(
                    "SELECT id, title, mode, message_count FROM conversations WHERE workspace_hash = '' OR workspace_hash IS NULL",
                )
                .expect("准备查询失败");
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                })
                .expect("查询失败");

            println!("\n将要删除的记录:");
            for row in rows {
                if let Ok((id, title, mode, count)) = row {
                    println!("  - [{}] {} ({} 条消息, 模式: {})", id, title, count, mode);
                }
            }
        }

        // 确认删除
        print!("\n确认删除这 {} 条记录吗？(y/N): ", orphan_count);
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("已取消。");
            return;
        }

        // 执行级联删除
        let tx = match conn.unchecked_transaction() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("开启事务失败: {}", e);
                std::process::exit(1);
            }
        };

        // 1. 删除向量索引
        let vec_deleted = tx
            .execute(
                "DELETE FROM vec_messages WHERE rowid IN (
                    SELECT m.rowid FROM messages m
                    INNER JOIN conversations c ON m.conv_id = c.id
                    WHERE c.workspace_hash = '' OR c.workspace_hash IS NULL
                )",
                [],
            )
            .unwrap_or(0);
        println!("已删除 {} 条向量索引", vec_deleted);

        // 2. 删除消息
        let msg_deleted = tx
            .execute(
                "DELETE FROM messages WHERE conv_id IN (
                    SELECT id FROM conversations WHERE workspace_hash = '' OR workspace_hash IS NULL
                )",
                [],
            )
            .unwrap_or(0);
        println!("已删除 {} 条消息", msg_deleted);

        // 3. 删除会话
        let conv_deleted = tx
            .execute(
                "DELETE FROM conversations WHERE workspace_hash = '' OR workspace_hash IS NULL",
                [],
            )
            .unwrap_or(0);
        println!("已删除 {} 条会话", conv_deleted);

        if let Err(e) = tx.commit() {
            eprintln!("提交事务失败: {}", e);
            std::process::exit(1);
        }

        println!(
            "\n清理完成！共删除 {} 条无工作区绑定的历史会话。",
            conv_deleted
        );
    }
}
