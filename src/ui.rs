use crossterm::{cursor, terminal, ExecutableCommand};
use std::io::{self, BufRead, Write, stdout};

pub fn clear_screen() {
    let _ = stdout().execute(terminal::Clear(terminal::ClearType::All));
    let _ = stdout().execute(cursor::MoveTo(0, 0));
}

pub fn print_header() {
    print_separator();
    println!("  ██████╗██╗      ██████╗ ");
    println!(" ██╔════╝██║     ██╔════╝ ");
    println!(" ██║     ██║     ██║  ███╗");
    println!(" ██║     ██║     ██║   ██║");
    println!(" ╚██████╗███████╗╚██████╔╝");
    println!("  ╚═════╝╚══════╝ ╚═════╝ ");
    println!("     命令行游戏  v0.1    ");
    print_separator();
}

#[allow(dead_code)]
pub fn print_player_status(player: &crate::engine::player::Player) {
    print_separator();
    println!("角色：{}  |  等级：{}", player.name, player.level);
    println!(
        "生命值：{}/{}  |  攻击：{}  |  防御：{}",
        player.hp, player.max_hp, player.attack, player.defense
    );
    println!(
        "经验：{}/{}  |  金币：{}g",
        player.exp, player.exp_to_next, player.gold
    );
    print_separator();
}

pub fn print_menu(title: &str, options: &[&str]) -> usize {
    loop {
        println!("\n=== {} ===", title);
        for (i, opt) in options.iter().enumerate() {
            println!("  {}. {}", i + 1, opt);
        }
        print!("选择：");
        let _ = stdout().flush();

        let line = read_line();
        if let Ok(n) = line.trim().parse::<usize>() {
            if n >= 1 && n <= options.len() {
                return n - 1;
            }
        }
        println!("无效选择，请输入 1-{}。", options.len());
    }
}

pub fn print_message(msg: &str) {
    println!("{}", msg);
}

#[allow(dead_code)]
pub fn wait_for_enter() {
    print!("\n按 Enter 继续...");
    let _ = stdout().flush();
    let stdin = io::stdin();
    let _ = stdin.lock().lines().next();
}

pub fn read_line() -> String {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).unwrap_or(0);
    line.trim_end_matches('\n').trim_end_matches('\r').to_string()
}

pub fn print_separator() {
    println!("{}", "─".repeat(50));
}
