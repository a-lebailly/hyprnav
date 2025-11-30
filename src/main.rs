use regex::Regex;
use serde::Deserialize;
use std::cmp::Ordering;
use std::env;
use std::f64::INFINITY;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct Workspace {
    id: i32,
}

#[derive(Debug, Deserialize)]
struct Client {
    address: String,
    at: [f64; 2],
    size: [f64; 2],
    workspace: Workspace,
}

fn run_hyprctl(args: &[&str]) -> String {
    let output = match Command::new("hyprctl").args(args).output() {
        Ok(o) => o,
        Err(_) => return String::new(),
    };

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn get_clients() -> Vec<Client> {
    let out = run_hyprctl(&["clients", "-j"]);
    if out.is_empty() {
        return Vec::new();
    }

    serde_json::from_str::<Vec<Client>>(&out).unwrap_or_else(|_| Vec::new())
}

fn get_active() -> Option<Client> {
    let out = run_hyprctl(&["activewindow", "-j"]);
    if out.is_empty() {
        return None;
    }

    serde_json::from_str::<Client>(&out).ok()
}

fn center_and_rect(c: &Client) -> (f64, f64, f64, f64, f64, f64) {
    let x = c.at[0];
    let y = c.at[1];
    let w = c.size[0];
    let h = c.size[1];

    let cx = x + w / 2.0;
    let cy = y + h / 2.0;

    (cx, cy, x, y, x + w, y + h)
}

fn overlap_1d(a1: f64, a2: f64, b1: f64, b2: f64) -> f64 {
    (a2.min(b2) - a1.max(b1)).max(0.0)
}

// <- ICI : lifetimes explicites
fn pick_target<'a>(
    direction: &str,
    active: &'a Client,
    clients: &'a [Client],
) -> Option<&'a Client> {
    let aw = active.workspace.id;
    let (axc, ayc, ax1, ay1, ax2, ay2) = center_and_rect(active);

    let mut best_index: Option<usize> = None;
    let mut best_score = (INFINITY, INFINITY);

    for (i, c) in clients.iter().enumerate() {
        if c.address == active.address {
            continue;
        }
        if c.workspace.id != aw {
            continue;
        }

        let (cxc, cyc, cx1, cy1, cx2, cy2) = center_and_rect(c);

        let (primary, secondary) = match direction {
            "right" => {
                if cxc <= axc {
                    continue;
                }
                if overlap_1d(ay1, ay2, cy1, cy2) <= 0.0 {
                    continue;
                }
                (cxc - axc, (cyc - ayc).abs())
            }
            "left" => {
                if cxc >= axc {
                    continue;
                }
                if overlap_1d(ay1, ay2, cy1, cy2) <= 0.0 {
                    continue;
                }
                (axc - cxc, (cyc - ayc).abs())
            }
            "down" => {
                if cyc <= ayc {
                    continue;
                }
                if overlap_1d(ax1, ax2, cx1, cx2) <= 0.0 {
                    continue;
                }
                (cyc - ayc, (cxc - axc).abs())
            }
            "up" => {
                if cyc >= ayc {
                    continue;
                }
                if overlap_1d(ax1, ax2, cx1, cx2) <= 0.0 {
                    continue;
                }
                (ayc - cyc, (cxc - axc).abs())
            }
            _ => return None,
        };

        let score = (primary, secondary);

        if score_cmp(score, best_score) == Ordering::Less {
            best_score = score;
            best_index = Some(i);
        }
    }

    best_index.map(|i| &clients[i])
}

fn score_cmp(a: (f64, f64), b: (f64, f64)) -> Ordering {
    match a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal) {
        Ordering::Equal => a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal),
        ord => ord,
    }
}

fn focus_client(c: Option<&Client>) {
    if let Some(client) = c {
        let arg = format!("address:{}", client.address);
        let args = ["dispatch", "focuswindow", arg.as_str()];
        run_hyprctl(&args);
    }
}

fn hyprland_config_path() -> PathBuf {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let mut p = PathBuf::from(home);
    p.push(".config/hypr/hyprland.conf");
    p
}

fn find_original_movefocus_binds() -> Vec<String> {
    let path = hyprland_config_path();
    if !path.exists() {
        return Vec::new();
    }

    let file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);

    let re = Regex::new(
        r"^\s*bind\s*=\s*\$mainMod\s*,\s*([^,]+)\s*,\s*movefocus\s*,\s*([lrud])\b",
    )
    .unwrap();

    let mut bindings = Vec::new();

    for line in reader.lines().flatten() {
        if let Some(caps) = re.captures(&line) {
            let key = caps.get(1).unwrap().as_str().trim();
            let direction = caps.get(2).unwrap().as_str().trim();
            bindings.push(format!("$mainMod, {key}, movefocus, {direction}"));
        }
    }

    bindings
}

fn enable_binds() {
    println!("Applying hyprnav directional bindings...");

    // Unbind Super+arrows
    for key in ["right", "left", "up", "down"] {
        let combo = format!("SUPER, {key}");
        let args = ["keyword", "unbind", combo.as_str()];
        run_hyprctl(&args);
    }

    let original_binds = find_original_movefocus_binds();

    if !original_binds.is_empty() {
        let direction_map = [("l", "left"), ("r", "right"), ("u", "up"), ("d", "down")];

        for bind in original_binds {
            let parts: Vec<_> = bind.split(',').map(|s| s.trim()).collect();
            if parts.len() != 4 {
                continue;
            }

            let key = parts[1];
            let dir_char = parts[3];

            let dir_word = direction_map
                .iter()
                .find(|(c, _)| *c == dir_char) // <- ICI : on enlève le &
                .map(|(_, w)| *w);

            let Some(dir_word) = dir_word else {
                continue;
            };

            // Unbind original key
            let unbind_arg = format!("$mainMod, {key}");
            let unbind_args = ["keyword", "unbind", unbind_arg.as_str()];
            run_hyprctl(&unbind_args);

            // Bind to hyprnav
            let bind_arg = format!("$mainMod, {key}, exec, hyprnav {dir_word}");
            let bind_args = ["keyword", "bind", bind_arg.as_str()];
            run_hyprctl(&bind_args);
        }

        println!("hyprnav bindings applied using keys from ~/.config/hypr/hyprland.conf.");
    } else {
        // Fallback: SUPER+arrows
        let binds = [
            "SUPER, right, exec, hyprnav right",
            "SUPER, left, exec, hyprnav left",
            "SUPER, up, exec, hyprnav up",
            "SUPER, down, exec, hyprnav down",
        ];

        for b in binds {
            let args = ["keyword", "bind", b];
            run_hyprctl(&args);
        }

        println!(
            "No original movefocus bindings found. Applied default SUPER+arrow hyprnav bindings."
        );
    }
}

fn disable_binds() {
    println!("Restoring original Hyprland focus bindings...");

    for key in ["right", "left", "up", "down"] {
        let combo = format!("SUPER, {key}");
        let args = ["keyword", "unbind", combo.as_str()];
        run_hyprctl(&args);
    }

    let original_binds = find_original_movefocus_binds();

    if !original_binds.is_empty() {
        for bind in original_binds {
            let args = ["keyword", "bind", bind.as_str()];
            run_hyprctl(&args);
        }
        println!("Bindings restored from ~/.config/hypr/hyprland.conf.");
    } else {
        let default_binds = [
            "$mainMod, left, movefocus, l",
            "$mainMod, right, movefocus, r",
            "$mainMod, up, movefocus, u",
            "$mainMod, down, movefocus, d",
        ];

        for b in default_binds {
            let args = ["keyword", "bind", b];
            run_hyprctl(&args);
        }

        println!("No original movefocus bindings found. Applied default movefocus bindings.");
    }

    println!("Bindings restored.");
}

fn show_help() {
    println!("hyprnav – directional focus navigation for Hyprland\n");
    println!("Usage:");
    println!("  hyprnav <command>\n");
    println!("Commands:");
    println!("  enable       Apply hyprnav directional bindings");
    println!("  disable      Restore Hyprland's default movefocus bindings");
    println!("  left         Focus the window to the left");
    println!("  right        Focus the window to the right");
    println!("  up           Focus the window above");
    println!("  down         Focus the window below");
    println!("  help         Show this help message");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        show_help();
        return;
    }

    let arg = args[1].to_lowercase();

    match arg.as_str() {
        "enable" => {
            enable_binds();
        }
        "disable" => {
            disable_binds();
        }
        "help" | "--help" | "-h" => {
            show_help();
        }
        "left" | "right" | "up" | "down" => {
            let active = match get_active() {
                Some(a) => a,
                None => return,
            };
            let clients = get_clients();
            let target = pick_target(&arg, &active, &clients);
            focus_client(target);
        }
        _ => {
            show_help();
        }
    }
}
