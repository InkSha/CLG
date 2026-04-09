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
    println!("  Command-Line Game  v0.1  ");
    print_separator();
}

pub fn print_player_status(player: &crate::player::Player) {
    print_separator();
    println!("Player: {}  |  Level: {}", player.name, player.level);
    println!(
        "HP: {}/{}  |  ATK: {}  |  DEF: {}",
        player.hp, player.max_hp, player.attack, player.defense
    );
    println!(
        "EXP: {}/{}  |  Gold: {}g",
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
        print!("Choice: ");
        let _ = stdout().flush();

        let line = read_line();
        if let Ok(n) = line.trim().parse::<usize>() {
            if n >= 1 && n <= options.len() {
                return n - 1;
            }
        }
        println!("Invalid choice. Please enter 1-{}.", options.len());
    }
}

pub fn print_message(msg: &str) {
    println!("{}", msg);
}

pub fn wait_for_enter() {
    print!("\nPress Enter to continue...");
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
