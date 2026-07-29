//! `fanuc_probe` — a read-only-by-default capability sweep for FANUC RMI
//! controllers.
//!
//! Point it at a controller and it walks the RMI surface: connect, status, the
//! `FRC_Initialize` group-mask discovery, every read command, registers, and
//! (only when explicitly asked) a set of deliberately tiny motions. Every frame
//! in both directions is written to a gitignored output directory, and the run
//! ends with a markdown capability report.
//!
//! ```text
//! cargo run -p fanuc_probe -- --addr 192.168.0.100:16001            # READ-ONLY
//! cargo run -p fanuc_probe -- --addr 192.168.0.100:16001 --motion   # + tiny motions
//! ```
//!
//! Safety rules baked in, not configurable:
//!   * nothing moves without `--motion` plus an explicit confirmation;
//!   * linear speed is capped at 1 mm/s and rotary at 0.5 deg/s (the CLI refuses
//!     faster values rather than silently clamping the operator's intent);
//!   * moves are 1 mm / 1°, always relative to a freshly read pose, always
//!     returning to where they started;
//!   * **the arm is never commanded in rotation** (see
//!     `Sweep::enforce_no_arm_rotation`);
//!   * any non-zero `ErrorID`, fault, or out-of-tolerance move aborts the
//!     sequence and disconnects cleanly.

mod capture;
mod report;
mod session;
mod sweep;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig, LogLevel};

use capture::Recorder;
use session::Session;
use sweep::{Findings, Sweep, MAX_LINEAR_SPEED_MM_S, MAX_ROTARY_SPEED_DEG_S};

#[derive(Parser, Debug, Clone)]
#[command(
    name = "fanuc_probe",
    about = "Read-only-by-default capability sweep / protocol probe for a FANUC RMI controller"
)]
pub struct Cli {
    /// Controller address as host:port (the RMI control port, normally 16001).
    #[arg(long, default_value = "127.0.0.1:16001")]
    pub addr: String,

    /// Root directory for captured output (gitignored).
    #[arg(long, default_value = "probe-output")]
    pub out: PathBuf,

    /// Enable the tiny motion sweep: 1 mm on X/Y/Z (Group 1, linear only) and
    /// ±1° on the Group-2 positioner. Requires confirmation.
    #[arg(long)]
    pub motion: bool,

    /// Non-interactive confirmation for --motion. Use this when an agent or a
    /// script runs the tool, so the intent is unambiguous in the typed command.
    #[arg(long)]
    pub i_have_verified_the_cell_is_clear: bool,

    /// Probe the non-motion lifecycle commands (pause / continue / reset).
    #[arg(long)]
    pub lifecycle: bool,

    /// Write-and-restore test on numeric register R[N].
    #[arg(long, value_name = "N")]
    pub write_register: Option<u16>,

    /// Run a controller-resident TP program by name. Requires --motion.
    #[arg(long, value_name = "NAME")]
    pub run_tp_program: Option<String>,

    /// Highest numeric register to read during the sweep.
    #[arg(long, default_value_t = 16)]
    pub register_sweep: u16,

    /// Skip the FRC_Initialize group-mask discovery (leaves the controller
    /// completely untouched, but the group verdict cannot be reached).
    #[arg(long)]
    pub skip_initialize: bool,

    /// Linear speed in mm/s. Capped at 1.0.
    #[arg(long, default_value_t = MAX_LINEAR_SPEED_MM_S)]
    pub linear_speed: f64,

    /// Rotary (positioner) speed in deg/s. Capped at 0.5.
    #[arg(long, default_value_t = MAX_ROTARY_SPEED_DEG_S)]
    pub rotary_speed: f64,

    /// Response timeout for ordinary commands.
    #[arg(long, default_value_t = 5)]
    pub timeout_secs: u64,

    /// Completion timeout for motion instructions. On timeout the probe sends
    /// FRC_Abort immediately, so keep this short: a 1 mm move at 1 mm/s takes
    /// about a second.
    #[arg(long, default_value_t = 20)]
    pub motion_timeout_secs: u64,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // --- speed ceilings: refuse, don't silently clamp -----------------------
    if !(cli.linear_speed > 0.0) || cli.linear_speed > MAX_LINEAR_SPEED_MM_S {
        eprintln!(
            "refusing to run: --linear-speed must be > 0 and <= {MAX_LINEAR_SPEED_MM_S} mm/s (got {})",
            cli.linear_speed
        );
        return ExitCode::from(2);
    }
    if !(cli.rotary_speed > 0.0) || cli.rotary_speed > MAX_ROTARY_SPEED_DEG_S {
        eprintln!(
            "refusing to run: --rotary-speed must be > 0 and <= {MAX_ROTARY_SPEED_DEG_S} deg/s (got {})",
            cli.rotary_speed
        );
        return ExitCode::from(2);
    }

    let (host, port) = match split_addr(&cli.addr) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("refusing to run: {e}");
            return ExitCode::from(2);
        }
    };

    banner(&cli, &host, port);

    if cli.motion && !confirm_motion(&cli) {
        eprintln!("Motion not confirmed — nothing was sent to the controller.");
        return ExitCode::from(3);
    }

    // --- capture ------------------------------------------------------------
    let rec = match Recorder::new(&cli.out) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("could not create the output directory under {:?}: {e}", cli.out);
            return ExitCode::FAILURE;
        }
    };
    println!("Capturing to {}\n", rec.dir().display());

    // --- connect ------------------------------------------------------------
    let config = FanucDriverConfig {
        addr: host.clone(),
        port,
        max_messages: 30,
        log_level: LogLevel::Warn,
    };
    rec.begin_step("000-FRC_Connect");
    let driver = match FanucDriver::connect_with_raw_hook(config, rec.hook()).await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("\nFailed to connect to {}: {e}", cli.addr);
            let _ = rec.write_steps();
            return ExitCode::FAILURE;
        }
    };
    let connect_frames = rec.frames_for("000-FRC_Connect");
    let connect_response = connect_frames
        .iter()
        .find(|f| f.is_received())
        .and_then(|f| f.json.clone());
    println!("[  ok  ]   0. FRC_Connect                 handshake with {}", cli.addr);

    let mut findings = Findings {
        addr: cli.addr.clone(),
        started_utc: capture::utc_rfc3339(capture::now_unix_ms()),
        mode: if cli.motion { "MOTION ENABLED".into() } else { "READ-ONLY".into() },
        connect: connect_response,
        ..Default::default()
    };
    findings.motion.linear_speed_mm_s = cli.linear_speed;
    findings.motion.rotary_speed_deg_s = cli.rotary_speed;

    let session = Session {
        driver: driver.clone(),
        rec: rec.clone(),
        timeout: Duration::from_secs(cli.timeout_secs),
    };

    // --- sweep --------------------------------------------------------------
    let mut sweep = Sweep::new(&session, &cli, findings);
    sweep.inventory().await;
    sweep.initialize_discovery().await;
    sweep.lifecycle().await;
    sweep.register_write().await;
    sweep.motion().await;
    sweep.tp_program().await;
    sweep.teardown().await;
    let findings = sweep.findings;

    // --- post-processing ----------------------------------------------------
    let steps = rec.steps();
    let frames = rec.frames();
    let _ = rec.write_steps();
    if let Ok(summary) = serde_json::to_string_pretty(&findings) {
        let _ = rec.write_file("summary.json", &summary);
    }
    let markdown = report::build(&findings, &steps, &frames);
    let report_path = rec.write_file("capability-report.md", &markdown);

    let ok = steps.iter().filter(|s| s.succeeded()).count();
    let failed = steps
        .iter()
        .filter(|s| matches!(s.status.as_str(), "controller_error" | "deserialize_failed" | "timeout" | "send_failed"))
        .count();
    println!("\n{}", "-".repeat(72));
    println!("{ok} exchange(s) succeeded, {failed} did not, {} raw frame(s) captured.", frames.len());
    println!("Output directory: {}", rec.dir().display());
    if let Ok(p) = report_path {
        println!("Capability report: {}", p.display());
    }

    if findings.motion.aborted {
        return ExitCode::from(4);
    }
    ExitCode::SUCCESS
}

fn banner(cli: &Cli, host: &str, port: u32) {
    let mode = if cli.motion { "MOTION ENABLED" } else { "READ-ONLY" };
    println!("{}", "=".repeat(72));
    println!("  FANUC RMI capability probe");
    println!("{}", "=".repeat(72));
    println!("  Target            : {host}:{port}");
    println!("  Mode              : {mode}");
    if cli.motion {
        println!("  Linear motion     : ±{:.0} mm on X, Y, Z (Group 1) @ {} mm/s", sweep::LINEAR_STEP_MM, cli.linear_speed);
        println!("  Rotary motion     : ±{:.0}° on Group 2 (positioner) @ {} deg/s", sweep::ROTARY_STEP_DEG, cli.rotary_speed);
        println!("  Arm rotation      : FORBIDDEN — W/P/R are never commanded on Group 1");
        println!("  Speed ceilings    : {MAX_LINEAR_SPEED_MM_S} mm/s linear, {MAX_ROTARY_SPEED_DEG_S} deg/s rotary (enforced)");
        println!("  Every move is relative to a freshly read pose and returns to it.");
        println!("  Positioner moves go out as two-group packets with a ZERO G1 block; the arm's");
        println!("  joints are re-read after each one and the sweep stops if the arm moved at all.");
        println!("  Any error, out-of-tolerance move, or {}s motion timeout triggers FRC_Abort.", cli.motion_timeout_secs);
    } else {
        println!("  Motion            : DISABLED — no motion packet will be sent");
    }
    println!("  Initialize probe  : {}", if cli.skip_initialize { "skipped" } else { "GroupMask 0b01, 0b11, 0b10 (mutates RMI session state)" });
    println!("  Lifecycle probe   : {}", if cli.lifecycle { "enabled (pause/continue/reset)" } else { "disabled" });
    println!("  Register write    : {}", cli.write_register.map(|r| format!("R[{r}] (restored afterwards)")).unwrap_or_else(|| "disabled".into()));
    println!("  TP program        : {}", cli.run_tp_program.clone().unwrap_or_else(|| "none".into()));
    println!("{}", "=".repeat(72));
    println!();
}

/// `--motion` needs an unambiguous yes: either the explicit flag, or a typed
/// confirmation at the terminal.
fn confirm_motion(cli: &Cli) -> bool {
    if cli.i_have_verified_the_cell_is_clear {
        println!("Motion confirmed via --i-have-verified-the-cell-is-clear.\n");
        return true;
    }
    print!("The robot WILL move (1 mm per axis, 1° on the positioner). Is the cell clear? Type YES to proceed: ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => {
            eprintln!("\nNo terminal available to confirm. Re-run with --i-have-verified-the-cell-is-clear.");
            false
        }
        Ok(_) => {
            let answer = line.trim() == "YES";
            println!();
            answer
        }
    }
}

fn split_addr(addr: &str) -> Result<(String, u32), String> {
    let (host, port) = addr
        .rsplit_once(':')
        .ok_or_else(|| format!("--addr must be host:port (got {addr:?})"))?;
    if host.is_empty() {
        return Err(format!("--addr is missing a host (got {addr:?})"));
    }
    let port: u32 = port
        .parse()
        .map_err(|_| format!("--addr has a non-numeric port (got {addr:?})"))?;
    Ok((host.to_string(), port))
}
