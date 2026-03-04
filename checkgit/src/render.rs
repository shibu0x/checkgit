use crate::helper::{clear_screen, move_cursor_right, move_cursor_up};
use colored::Colorize;
use viuer::{Config, print};

pub fn render(profile: checkgit_core::UserProfile) {
    clear_screen();

    let avatar_width: u32 = 25;

    print(
        &profile.avatar_image,
        &Config {
            width: Some(avatar_width),
            use_kitty: true,
            use_iterm: true,
            absolute_offset: false,
            ..Default::default()
        },
    )
    .unwrap();

    let avatar_height_rows = (avatar_width / 2) + 2;

    move_cursor_up(avatar_height_rows as u16);

    let col: u16 = (avatar_width + 4) as u16;

    let name = profile
        .display_name
        .clone()
        .unwrap_or(profile.username.clone());

    move_cursor_right(col);
    println!("{}", name.bold().truecolor(230, 237, 243));

    move_cursor_right(col);
    println!(
        "{}",
        format!("@{}", profile.username).truecolor(125, 133, 144)
    );

    if let Some(ref bio) = profile.bio {
        move_cursor_right(col);
        println!("{}", bio.truecolor(173, 186, 199));
    }

    move_cursor_right(col);
    println!();

    move_cursor_right(col);
    print!("{} ", "◉".truecolor(125, 133, 144));
    print!(
        "{} ",
        profile
            .followers
            .to_string()
            .bold()
            .truecolor(230, 237, 243)
    );
    print!("{}", "followers".truecolor(125, 133, 144));
    print!("  {}  ", "·".truecolor(48, 54, 61));
    print!(
        "{} ",
        profile
            .following
            .to_string()
            .bold()
            .truecolor(230, 237, 243)
    );
    println!("{}", "following".truecolor(125, 133, 144));

    move_cursor_right(col);
    print!("{} ", "⊞".truecolor(125, 133, 144));
    print!(
        "{} ",
        profile
            .repo_count
            .to_string()
            .bold()
            .truecolor(230, 237, 243)
    );
    print!("{}", "repos".truecolor(125, 133, 144));
    print!("    ");
    print!("{} ", "★".truecolor(210, 153, 34));
    print!(
        "{} ",
        profile
            .total_stars
            .to_string()
            .bold()
            .truecolor(230, 237, 243)
    );
    println!("{}", "stars".truecolor(125, 133, 144));

    move_cursor_right(col);
    println!();

    move_cursor_right(col);
    println!("{}", "Popular repositories".bold().truecolor(230, 237, 243));
    move_cursor_right(col);
    println!("{}", "─".repeat(34).truecolor(48, 54, 61));

    for (repo, stars) in &profile.top_repos {
        move_cursor_right(col);
        print!("{} ", "◈".truecolor(88, 166, 255));
        print!("{:<24}", repo.truecolor(88, 166, 255));
        print!("{} ", "★".truecolor(210, 153, 34));
        println!("{}", stars.to_string().truecolor(125, 133, 144));
    }

    let profile_lines_printed: u16 =
        10 + profile.top_repos.len() as u16 + if profile.bio.is_some() { 1 } else { 0 };

    let remaining = (avatar_height_rows as u16).saturating_sub(profile_lines_printed);
    for _ in 0..remaining {
        println!();
    }

    println!();
    render_heatmap(profile.contribution_matrix);
}

pub fn render_heatmap(matrix: Vec<Vec<u32>>) {
    if matrix.is_empty() {
        return;
    }

    let weeks = matrix.iter().map(|r| r.len()).max().unwrap_or(0);
    let total: u32 = matrix.iter().flatten().sum();

    println!(
        "{} contributions in the last year\n",
        total.to_string().bold().truecolor(230, 237, 243)
    );

    fn level_color(level: u8) -> (u8, u8, u8) {
        match level {
            0 => (80, 80, 80),
            1 => (0, 120, 0),
            2 => (0, 160, 0),
            3 => (0, 200, 0),
            _ => (0, 255, 0),
        }
    }

    fn level(value: u32) -> u8 {
        match value {
            0 => 0,
            1..=3 => 1,
            4..=7 => 2,
            8..=15 => 3,
            _ => 4,
        }
    }

    let months = [
        "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "Jan", "Feb", "Mar",
    ];

    let mut month_positions = vec![None; weeks];

    for (i, m) in months.iter().enumerate() {
        let pos = (i * weeks) / 12;
        if pos < weeks {
            month_positions[pos] = Some(*m);
        }
    }

    print!("     ");

    for w in 0..weeks {
        if let Some(m) = month_positions[w] {
            print!("{}", m.truecolor(125, 133, 144));
            for _ in 0..(3 - m.len()) {
                print!(" ");
            }
        } else {
            print!("   ");
        }
    }

    println!();

    for day in 0..7 {
        let label = match day {
            1 => "Mon",
            3 => "Wed",
            5 => "Fri",
            _ => "   ",
        };

        print!("{} ", label.truecolor(125, 133, 144));

        if let Some(row) = matrix.get(day) {
            for value in row {
                let lvl = level(*value);
                let (r, g, b) = level_color(lvl);

                print!("{}", "██".truecolor(r, g, b));
                print!(" ");
            }
        }

        println!();
    }


    println!();

    print!("{}", "Less ".truecolor(125, 133, 144));

    for lvl in 0..=4 {
        let (r, g, b) = level_color(lvl);
        print!("{}", "  ".on_truecolor(r, g, b));
        print!(" ");
    }

    println!("{}", "More".truecolor(125, 133, 144));


    let mut longest = 0;
    let mut current = 0;
    let mut running = 0;

    for week in 0..weeks {
        for day in 0..7 {
            let v = matrix
                .get(day)
                .and_then(|r| r.get(week))
                .copied()
                .unwrap_or(0);

            if v > 0 {
                running += 1;
                longest = longest.max(running);
            } else {
                running = 0;
            }
        }
    }

    for week in (0..weeks).rev() {
        for day in (0..7).rev() {
            let v = matrix
                .get(day)
                .and_then(|r| r.get(week))
                .copied()
                .unwrap_or(0);

            if v > 0 {
                current += 1;
            } else {
                break;
            }
        }
        if current == 0 {
            break;
        }
    }

    println!();
    println!(
        "{} {}   {} {}",
        "Current streak:".truecolor(125, 133, 144),
        current.to_string().bold(),
        "Longest streak:".truecolor(125, 133, 144),
        longest.to_string().bold()
    );

    println!();
}
