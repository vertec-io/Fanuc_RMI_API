//! Robot Creation Wizard - Production-grade multi-step wizard for creating robots with configurations.
//!
//! This wizard guides users through creating a robot connection with at least one configuration.
//! It ensures data integrity and provides excellent UX with validation, progress tracking, and exit warnings.

use leptos::prelude::*;
use crate::websocket::{WebSocketManager, NewRobotConfigurationDto};

/// Wizard step enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WizardStep {
    ConnectionDetails,
    MotionDefaults,
    JogDefaults,
    DefaultConfiguration,
    AdditionalConfigurations,
}

impl WizardStep {
    fn step_number(&self) -> usize {
        match self {
            WizardStep::ConnectionDetails => 1,
            WizardStep::MotionDefaults => 2,
            WizardStep::JogDefaults => 3,
            WizardStep::DefaultConfiguration => 4,
            WizardStep::AdditionalConfigurations => 5,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            WizardStep::ConnectionDetails => "Connection Details",
            WizardStep::MotionDefaults => "Motion Defaults",
            WizardStep::JogDefaults => "Jog Defaults",
            WizardStep::DefaultConfiguration => "Default Configuration",
            WizardStep::AdditionalConfigurations => "Additional Configurations",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            WizardStep::ConnectionDetails => "Enter robot name, IP address, and connection details",
            WizardStep::MotionDefaults => "Set default motion parameters for this robot",
            WizardStep::JogDefaults => "Configure jogging speeds and step sizes",
            WizardStep::DefaultConfiguration => "Create the default configuration (required)",
            WizardStep::AdditionalConfigurations => "Add more configurations (optional)",
        }
    }
}

/// Robot Creation Wizard Component
#[component]
pub fn RobotCreationWizard<F1, F2>(
    on_close: F1,
    on_created: F2,
) -> impl IntoView
where
    F1: Fn(i64) + Clone + Send + Sync + 'static,
    F2: Fn(i64) + Clone + Send + Sync + 'static,
{
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    // Wizard state
    let (current_step, set_current_step) = signal(WizardStep::ConnectionDetails);
    let (show_exit_warning, set_show_exit_warning) = signal(false);
    let (validation_error, set_validation_error) = signal::<Option<String>>(None);
    let (is_submitting, set_is_submitting) = signal(false);

    // Step 1: Connection Details
    let (robot_name, set_robot_name) = signal(String::new());
    let (robot_description, set_robot_description) = signal(String::new());
    let (robot_ip, set_robot_ip) = signal("127.0.0.1".to_string());
    let (robot_port, set_robot_port) = signal("16001".to_string());

    // Step 2: Motion Defaults
    let (default_speed, set_default_speed) = signal("100.0".to_string());
    let (default_speed_type, set_default_speed_type) = signal("mmSec".to_string());
    let (default_term_type, set_default_term_type) = signal("CNT".to_string());
    let (default_w, set_default_w) = signal("0.0".to_string());
    let (default_p, set_default_p) = signal("0.0".to_string());
    let (default_r, set_default_r) = signal("0.0".to_string());

    // Step 3: Jog Defaults (safer defaults)
    let (cartesian_jog_speed, set_cartesian_jog_speed) = signal("10.0".to_string());
    let (cartesian_jog_step, set_cartesian_jog_step) = signal("1.0".to_string());
    let (joint_jog_speed, set_joint_jog_speed) = signal("0.1".to_string());
    let (joint_jog_step, set_joint_jog_step) = signal("0.25".to_string());

    // Step 4: Default Configuration
    let (config_name, set_config_name) = signal("Default".to_string());
    let (config_uframe, set_config_uframe) = signal("0".to_string());
    let (config_utool, set_config_utool) = signal("1".to_string());
    let (config_front, set_config_front) = signal(1);
    let (config_up, set_config_up) = signal(1);
    let (config_left, set_config_left) = signal(0);
    let (config_flip, set_config_flip) = signal(0);
    let (config_turn4, set_config_turn4) = signal("0".to_string());
    let (config_turn5, set_config_turn5) = signal("0".to_string());
    let (config_turn6, set_config_turn6) = signal("0".to_string());

    // Step 5: Additional Configurations
    let (configurations, set_configurations) = signal::<Vec<NewRobotConfigurationDto>>(Vec::new());
    let (editing_additional, set_editing_additional) = signal(false);

    // Validation helper
    let validate_current_step = move || -> Result<(), String> {
        match current_step.get() {
            WizardStep::ConnectionDetails => {
                if robot_name.get().trim().is_empty() {
                    return Err("Robot name is required".to_string());
                }
                if robot_ip.get().trim().is_empty() {
                    return Err("IP address is required".to_string());
                }
                robot_port.get().parse::<u32>()
                    .map_err(|_| "Invalid port number".to_string())?;
                Ok(())
            }
            WizardStep::MotionDefaults => {
                let speed = default_speed.get().parse::<f64>()
                    .map_err(|_| "Invalid speed value".to_string())?;
                if !(0.0..=100.0).contains(&speed) {
                    return Err("Speed must be between 0 and 100".to_string());
                }
                default_w.get().parse::<f64>()
                    .map_err(|_| "Invalid W value".to_string())?;
                default_p.get().parse::<f64>()
                    .map_err(|_| "Invalid P value".to_string())?;
                default_r.get().parse::<f64>()
                    .map_err(|_| "Invalid R value".to_string())?;
                Ok(())
            }
            WizardStep::JogDefaults => {
                cartesian_jog_speed.get().parse::<f64>()
                    .map_err(|_| "Invalid cartesian jog speed".to_string())?;
                cartesian_jog_step.get().parse::<f64>()
                    .map_err(|_| "Invalid cartesian jog step".to_string())?;
                let joint_speed = joint_jog_speed.get().parse::<f64>()
                    .map_err(|_| "Invalid joint jog speed".to_string())?;
                if !(0.0..=100.0).contains(&joint_speed) {
                    return Err("Joint jog speed must be between 0 and 100".to_string());
                }
                joint_jog_step.get().parse::<f64>()
                    .map_err(|_| "Invalid joint jog step".to_string())?;
                Ok(())
            }
            WizardStep::DefaultConfiguration => {
                if config_name.get().trim().is_empty() {
                    return Err("Configuration name is required".to_string());
                }
                let uframe = config_uframe.get().parse::<i32>()
                    .map_err(|_| "Invalid UFrame number".to_string())?;
                if !(0..=9).contains(&uframe) {
                    return Err("UFrame must be between 0 and 9".to_string());
                }
                let utool = config_utool.get().parse::<i32>()
                    .map_err(|_| "Invalid UTool number".to_string())?;
                if !(1..=10).contains(&utool) {
                    return Err("UTool must be between 1 and 10 (Tool 0 is invalid)".to_string());
                }
                config_turn4.get().parse::<i32>()
                    .map_err(|_| "Invalid Turn 4 value".to_string())?;
                config_turn5.get().parse::<i32>()
                    .map_err(|_| "Invalid Turn 5 value".to_string())?;
                config_turn6.get().parse::<i32>()
                    .map_err(|_| "Invalid Turn 6 value".to_string())?;
                Ok(())
            }
            WizardStep::AdditionalConfigurations => Ok(()),
        }
    };

    // Submit handler
    let submit_robot = move || {
        if is_submitting.get() {
            return;
        }

        // Validate all steps
        set_validation_error.set(None);
        set_current_step.set(WizardStep::ConnectionDetails);
        if let Err(e) = validate_current_step() {
            set_validation_error.set(Some(e));
            return;
        }
        set_current_step.set(WizardStep::MotionDefaults);
        if let Err(e) = validate_current_step() {
            set_validation_error.set(Some(e));
            return;
        }
        set_current_step.set(WizardStep::JogDefaults);
        if let Err(e) = validate_current_step() {
            set_validation_error.set(Some(e));
            return;
        }
        set_current_step.set(WizardStep::DefaultConfiguration);
        if let Err(e) = validate_current_step() {
            set_validation_error.set(Some(e));
            return;
        }

        set_is_submitting.set(true);

        // Build the default configuration
        let default_config = NewRobotConfigurationDto {
            name: config_name.get(),
            is_default: true,
            u_frame_number: config_uframe.get().parse().unwrap(),
            u_tool_number: config_utool.get().parse().unwrap(),
            front: config_front.get(),
            up: config_up.get(),
            left: config_left.get(),
            flip: config_flip.get(),
            turn4: config_turn4.get().parse().unwrap(),
            turn5: config_turn5.get().parse().unwrap(),
            turn6: config_turn6.get().parse().unwrap(),
        };

        // Call the API
        ws.create_robot_with_configurations(
            robot_name.get(),
            if robot_description.get().is_empty() { None } else { Some(robot_description.get()) },
            robot_ip.get(),
            robot_port.get().parse().unwrap(),
            default_speed.get().parse().unwrap(),
            default_speed_type.get(),
            default_term_type.get(),
            default_w.get().parse().unwrap(),
            default_p.get().parse().unwrap(),
            default_r.get().parse().unwrap(),
            cartesian_jog_speed.get().parse().unwrap(),
            cartesian_jog_step.get().parse().unwrap(),
            joint_jog_speed.get().parse().unwrap(),
            joint_jog_step.get().parse().unwrap(),
            vec![default_config],
        );

        // Note: The actual response handling will be done via WebSocket message handler
        // For now, we'll just close the wizard after a short delay
        let on_created = on_created.clone();
        set_timeout(
            move || {
                set_is_submitting.set(false);
                on_created(0); // Will be updated when we get the actual ID from the server
            },
            std::time::Duration::from_millis(500),
        );
    };

    view! {
        <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
            <div class="bg-[#0d0d0d] border border-[#ffffff15] rounded-lg shadow-2xl w-full max-w-4xl max-h-[90vh] flex flex-col">
                // Header
                <WizardHeader current_step=current_step />

                // Content (scrollable)
                <div class="flex-1 overflow-y-auto p-6">
                    <Show when=move || current_step.get() == WizardStep::ConnectionDetails>
                        <ConnectionDetailsStep
                            robot_name=robot_name
                            set_robot_name=set_robot_name
                            robot_description=robot_description
                            set_robot_description=set_robot_description
                            robot_ip=robot_ip
                            set_robot_ip=set_robot_ip
                            robot_port=robot_port
                            set_robot_port=set_robot_port
                        />
                    </Show>

                    <Show when=move || current_step.get() == WizardStep::MotionDefaults>
                        <MotionDefaultsStep
                            default_speed=default_speed
                            set_default_speed=set_default_speed
                            default_speed_type=default_speed_type
                            set_default_speed_type=set_default_speed_type
                            default_term_type=default_term_type
                            set_default_term_type=set_default_term_type
                            default_w=default_w
                            set_default_w=set_default_w
                            default_p=default_p
                            set_default_p=set_default_p
                            default_r=default_r
                            set_default_r=set_default_r
                        />
                    </Show>

                    <Show when=move || current_step.get() == WizardStep::JogDefaults>
                        <JogDefaultsStep
                            cartesian_jog_speed=cartesian_jog_speed
                            set_cartesian_jog_speed=set_cartesian_jog_speed
                            cartesian_jog_step=cartesian_jog_step
                            set_cartesian_jog_step=set_cartesian_jog_step
                            joint_jog_speed=joint_jog_speed
                            set_joint_jog_speed=set_joint_jog_speed
                            joint_jog_step=joint_jog_step
                            set_joint_jog_step=set_joint_jog_step
                        />
                    </Show>

                    <Show when=move || current_step.get() == WizardStep::DefaultConfiguration>
                        <DefaultConfigurationStep
                            config_name=config_name
                            set_config_name=set_config_name
                            config_uframe=config_uframe
                            set_config_uframe=set_config_uframe
                            config_utool=config_utool
                            set_config_utool=set_config_utool
                            config_front=config_front
                            set_config_front=set_config_front
                            config_up=config_up
                            set_config_up=set_config_up
                            config_left=config_left
                            set_config_left=set_config_left
                            config_flip=config_flip
                            set_config_flip=set_config_flip
                            config_turn4=config_turn4
                            set_config_turn4=set_config_turn4
                            config_turn5=config_turn5
                            set_config_turn5=set_config_turn5
                            config_turn6=config_turn6
                            set_config_turn6=set_config_turn6
                        />
                    </Show>

                    <Show when=move || current_step.get() == WizardStep::AdditionalConfigurations>
                        <div class="text-center py-8">
                            <p class="text-[#888888] text-sm">"Additional configurations feature coming soon"</p>
                            <p class="text-[#666666] text-xs mt-2">"You can add more configurations after creating the robot"</p>
                        </div>
                    </Show>
                </div>

                // Footer with navigation
                <WizardFooter
                    current_step=current_step
                    set_current_step=set_current_step
                    set_show_exit_warning=set_show_exit_warning
                    validation_error=validation_error
                    set_validation_error=set_validation_error
                    validate_current_step=validate_current_step
                    submit_robot=submit_robot
                    is_submitting=is_submitting
                />
            </div>

            // Exit Warning Modal
            <Show when=move || show_exit_warning.get()>
                <ExitWarningModal
                    on_cancel=move || set_show_exit_warning.set(false)
                    on_confirm={
                        let on_close = on_close.clone();
                        move || {
                            set_show_exit_warning.set(false);
                            on_close(0);
                        }
                    }
                />
            </Show>
        </div>
    }
}

/// Wizard Header with progress indicator
#[component]
fn WizardHeader(current_step: ReadSignal<WizardStep>) -> impl IntoView {
    let steps = vec![
        WizardStep::ConnectionDetails,
        WizardStep::MotionDefaults,
        WizardStep::JogDefaults,
        WizardStep::DefaultConfiguration,
    ];

    view! {
        <div class="border-b border-[#ffffff08] p-6">
            <h2 class="text-lg font-semibold text-white mb-4">"Create New Robot"</h2>

            // Progress indicator
            <div class="flex items-center gap-2">
                <For
                    each=move || steps.clone().into_iter().enumerate()
                    key=|(idx, _)| *idx
                    children=move |(idx, step)| {
                        let is_current = move || current_step.get() == step;
                        let is_completed = move || current_step.get().step_number() > step.step_number();

                        view! {
                            <div class=move || format!(
                                "flex items-center justify-center w-8 h-8 rounded-full text-xs font-semibold transition-colors {}",
                                if is_current() {
                                    "bg-[#00d9ff] text-black"
                                } else if is_completed() {
                                    "bg-[#22c55e] text-black"
                                } else {
                                    "bg-[#1a1a1a] text-[#666666] border border-[#ffffff08]"
                                }
                            )>
                                {move || if is_completed() { "✓".to_string() } else { (idx + 1).to_string() }}
                            </div>

                            <Show when=move || idx < 3>
                                <div class=move || format!(
                                    "h-0.5 w-12 transition-colors {}",
                                    if is_completed() { "bg-[#22c55e]" } else { "bg-[#ffffff08]" }
                                )></div>
                            </Show>
                        }
                    }
                />
            </div>

            // Current step title and description
            <div class="mt-4">
                <h3 class="text-sm font-semibold text-white">{move || current_step.get().title()}</h3>
                <p class="text-xs text-[#888888] mt-1">{move || current_step.get().description()}</p>
            </div>
        </div>
    }
}

/// Connection Details Step
#[component]
fn ConnectionDetailsStep(
    robot_name: ReadSignal<String>,
    set_robot_name: WriteSignal<String>,
    robot_description: ReadSignal<String>,
    set_robot_description: WriteSignal<String>,
    robot_ip: ReadSignal<String>,
    set_robot_ip: WriteSignal<String>,
    robot_port: ReadSignal<String>,
    set_robot_port: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-4">
            <div>
                <label class="block text-[#888888] text-xs mb-1.5 font-medium">
                    "Robot Name" <span class="text-[#ef4444]">"*"</span>
                </label>
                <input
                    type="text"
                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors"
                    placeholder="e.g., Production Cell 1"
                    prop:value=move || robot_name.get()
                    on:input=move |ev| set_robot_name.set(event_target_value(&ev))
                />
                <p class="text-xs text-[#666666] mt-1">"A descriptive name for this robot"</p>
            </div>

            <div>
                <label class="block text-[#888888] text-xs mb-1.5 font-medium">"Description (optional)"</label>
                <input
                    type="text"
                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors"
                    placeholder="e.g., Main welding robot"
                    prop:value=move || robot_description.get()
                    on:input=move |ev| set_robot_description.set(event_target_value(&ev))
                />
            </div>

            <div class="grid grid-cols-2 gap-4">
                <div>
                    <label class="block text-[#888888] text-xs mb-1.5 font-medium">
                        "IP Address" <span class="text-[#ef4444]">"*"</span>
                    </label>
                    <input
                        type="text"
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                        placeholder="192.168.1.100"
                        prop:value=move || robot_ip.get()
                        on:input=move |ev| set_robot_ip.set(event_target_value(&ev))
                    />
                </div>

                <div>
                    <label class="block text-[#888888] text-xs mb-1.5 font-medium">
                        "Port" <span class="text-[#ef4444]">"*"</span>
                    </label>
                    <input
                        type="number"
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                        placeholder="16001"
                        prop:value=move || robot_port.get()
                        on:input=move |ev| set_robot_port.set(event_target_value(&ev))
                    />
                </div>
            </div>
        </div>
    }
}

/// Motion Defaults Step
#[component]
fn MotionDefaultsStep(
    default_speed: ReadSignal<String>,
    set_default_speed: WriteSignal<String>,
    default_speed_type: ReadSignal<String>,
    set_default_speed_type: WriteSignal<String>,
    default_term_type: ReadSignal<String>,
    set_default_term_type: WriteSignal<String>,
    default_w: ReadSignal<String>,
    set_default_w: WriteSignal<String>,
    default_p: ReadSignal<String>,
    set_default_p: WriteSignal<String>,
    default_r: ReadSignal<String>,
    set_default_r: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-4">
            <div class="grid grid-cols-2 gap-4">
                <div>
                    <label class="block text-[#888888] text-xs mb-1.5 font-medium">
                        "Default Speed" <span class="text-[#ef4444]">"*"</span>
                    </label>
                    <input
                        type="text"
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                        prop:value=move || default_speed.get()
                        on:input=move |ev| set_default_speed.set(event_target_value(&ev))
                        placeholder="100.0"
                    />
                    <p class="text-xs text-[#666666] mt-1">"Speed value (units depend on type)"</p>
                </div>

                <div>
                    <label class="block text-[#888888] text-xs mb-1.5 font-medium flex items-center gap-1.5">
                        <span>"Speed Type"</span>
                        <span class="text-[#ef4444]">"*"</span>
                        <span
                            class="inline-flex items-center justify-center w-3.5 h-3.5 rounded-full bg-[#00d9ff] text-[#0a0a0a] text-[10px] font-bold"
                            title="Speed Type determines how motion speed is interpreted:\n\n• mm/sec: Linear speed in millimeters per second (most common)\n• 0.1 inch/min: Linear speed in 0.1 inch per minute increments\n• 0.1 seconds: Time-based - motion completes in specified time (0.1 sec units)\n• milliseconds: Time-based - motion completes in specified milliseconds\n\nThis setting affects:\n✓ All motion commands (MOVE, MOVEJ, MOVEC)\n✓ Program execution\n✓ Quick commands from Dashboard\n\nNOTE: Individual commands can override this with their own speed/speedType parameters."
                        >
                            "?"
                        </span>
                    </label>
                    <select
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors"
                        prop:value=move || default_speed_type.get()
                        on:change=move |ev| set_default_speed_type.set(event_target_value(&ev))
                    >
                        <option value="mmSec">"mm/sec (Linear)"</option>
                        <option value="InchMin">"0.1 inch/min"</option>
                        <option value="Time">"0.1 seconds (Time-based)"</option>
                        <option value="mSec">"milliseconds"</option>
                    </select>
                    <p class="text-xs text-[#666666] mt-1">"How speed values are interpreted"</p>
                </div>
            </div>

            <div class="grid grid-cols-1 gap-4">
                <div>
                    <label class="block text-[#888888] text-xs mb-1.5 font-medium">
                        "Termination Type" <span class="text-[#ef4444]">"*"</span>
                    </label>
                    <select
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors"
                        prop:value=move || default_term_type.get()
                        on:change=move |ev| set_default_term_type.set(event_target_value(&ev))
                    >
                        <option value="CNT">"CNT (Continuous)"</option>
                        <option value="FINE">"FINE (Precise)"</option>
                    </select>
                    <p class="text-xs text-[#666666] mt-1">"Motion termination type"</p>
                </div>
            </div>

            <div class="border-t border-[#ffffff08] pt-4">
                <h4 class="text-sm font-semibold text-white mb-3">"Wrist Singularity Avoidance"</h4>
                <div class="grid grid-cols-3 gap-4">
                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">"W (Wrist)"</label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                            prop:value=move || default_w.get()
                            on:input=move |ev| set_default_w.set(event_target_value(&ev))
                            placeholder="0.0"
                        />
                    </div>

                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">"P (Pitch)"</label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                            prop:value=move || default_p.get()
                            on:input=move |ev| set_default_p.set(event_target_value(&ev))
                            placeholder="0.0"
                        />
                    </div>

                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">"R (Roll)"</label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                            prop:value=move || default_r.get()
                            on:input=move |ev| set_default_r.set(event_target_value(&ev))
                            placeholder="0.0"
                        />
                    </div>
                </div>
                <p class="text-xs text-[#666666] mt-2">"Typically 0.0 for all axes (no avoidance)"</p>
            </div>
        </div>
    }
}

/// Jog Defaults Step
#[component]
fn JogDefaultsStep(
    cartesian_jog_speed: ReadSignal<String>,
    set_cartesian_jog_speed: WriteSignal<String>,
    cartesian_jog_step: ReadSignal<String>,
    set_cartesian_jog_step: WriteSignal<String>,
    joint_jog_speed: ReadSignal<String>,
    set_joint_jog_speed: WriteSignal<String>,
    joint_jog_step: ReadSignal<String>,
    set_joint_jog_step: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="border border-[#ffffff08] rounded-lg p-4 bg-[#0a0a0a]">
                <h4 class="text-sm font-semibold text-white mb-3">"Cartesian Jogging"</h4>
                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">
                            "Speed (mm/s)" <span class="text-[#ef4444]">"*"</span>
                        </label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                            prop:value=move || cartesian_jog_speed.get()
                            on:input=move |ev| set_cartesian_jog_speed.set(event_target_value(&ev))
                            placeholder="10.0"
                        />
                        <p class="text-xs text-[#666666] mt-1">"Continuous jog speed"</p>
                    </div>

                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">
                            "Step Size (mm)" <span class="text-[#ef4444]">"*"</span>
                        </label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                            prop:value=move || cartesian_jog_step.get()
                            on:input=move |ev| set_cartesian_jog_step.set(event_target_value(&ev))
                            placeholder="1.0"
                        />
                        <p class="text-xs text-[#666666] mt-1">"Incremental jog distance"</p>
                    </div>
                </div>
            </div>

            <div class="border border-[#ffffff08] rounded-lg p-4 bg-[#0a0a0a]">
                <h4 class="text-sm font-semibold text-white mb-3">"Joint Jogging"</h4>
                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">
                            "Speed (%)" <span class="text-[#ef4444]">"*"</span>
                        </label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                            prop:value=move || joint_jog_speed.get()
                            on:input=move |ev| set_joint_jog_speed.set(event_target_value(&ev))
                            placeholder="0.1"
                        />
                        <p class="text-xs text-[#666666] mt-1">"Continuous jog speed"</p>
                    </div>

                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">
                            "Step Size (°)" <span class="text-[#ef4444]">"*"</span>
                        </label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                            prop:value=move || joint_jog_step.get()
                            on:input=move |ev| set_joint_jog_step.set(event_target_value(&ev))
                            placeholder="0.25"
                        />
                        <p class="text-xs text-[#666666] mt-1">"Incremental jog angle"</p>
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Default Configuration Step
#[component]
fn DefaultConfigurationStep(
    config_name: ReadSignal<String>,
    set_config_name: WriteSignal<String>,
    config_uframe: ReadSignal<String>,
    set_config_uframe: WriteSignal<String>,
    config_utool: ReadSignal<String>,
    set_config_utool: WriteSignal<String>,
    config_front: ReadSignal<i32>,
    set_config_front: WriteSignal<i32>,
    config_up: ReadSignal<i32>,
    set_config_up: WriteSignal<i32>,
    config_left: ReadSignal<i32>,
    set_config_left: WriteSignal<i32>,
    config_flip: ReadSignal<i32>,
    set_config_flip: WriteSignal<i32>,
    config_turn4: ReadSignal<String>,
    set_config_turn4: WriteSignal<String>,
    config_turn5: ReadSignal<String>,
    set_config_turn5: WriteSignal<String>,
    config_turn6: ReadSignal<String>,
    set_config_turn6: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-4">
            <div>
                <label class="block text-[#888888] text-xs mb-1.5 font-medium">
                    "Configuration Name" <span class="text-[#ef4444]">"*"</span>
                </label>
                <input
                    type="text"
                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors"
                    placeholder="e.g., Default"
                    prop:value=move || config_name.get()
                    on:input=move |ev| set_config_name.set(event_target_value(&ev))
                />
                <p class="text-xs text-[#666666] mt-1">"This will be the default configuration"</p>
            </div>

            <div class="grid grid-cols-2 gap-4">
                <div>
                    <label class="block text-[#888888] text-xs mb-1.5 font-medium">
                        "User Frame" <span class="text-[#ef4444]">"*"</span>
                    </label>
                    <input
                        type="text"
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                        prop:value=move || config_uframe.get()
                        on:input=move |ev| set_config_uframe.set(event_target_value(&ev))
                        placeholder="0"
                    />
                    <p class="text-xs text-[#666666] mt-1">"0-9 (0 = World)"</p>
                </div>

                <div>
                    <label class="block text-[#888888] text-xs mb-1.5 font-medium">
                        "User Tool" <span class="text-[#ef4444]">"*"</span>
                    </label>
                    <input
                        type="text"
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                        prop:value=move || config_utool.get()
                        on:input=move |ev| set_config_utool.set(event_target_value(&ev))
                        placeholder="1"
                    />
                    <p class="text-xs text-[#666666] mt-1">"1-10 (Tool 0 invalid)"</p>
                </div>
            </div>

            <div class="border-t border-[#ffffff08] pt-4">
                <h4 class="text-sm font-semibold text-white mb-3">"Arm Configuration"</h4>
                <div class="grid grid-cols-4 gap-3">
                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">"Front"</label>
                        <select
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors"
                            prop:value=move || config_front.get().to_string()
                            on:change=move |ev| set_config_front.set(event_target_value(&ev).parse().unwrap_or(1))
                        >
                            <option value="0">"0"</option>
                            <option value="1">"1"</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">"Up"</label>
                        <select
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors"
                            prop:value=move || config_up.get().to_string()
                            on:change=move |ev| set_config_up.set(event_target_value(&ev).parse().unwrap_or(1))
                        >
                            <option value="0">"0"</option>
                            <option value="1">"1"</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">"Left"</label>
                        <select
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors"
                            prop:value=move || config_left.get().to_string()
                            on:change=move |ev| set_config_left.set(event_target_value(&ev).parse().unwrap_or(0))
                        >
                            <option value="0">"0"</option>
                            <option value="1">"1"</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">"Flip"</label>
                        <select
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors"
                            prop:value=move || config_flip.get().to_string()
                            on:change=move |ev| set_config_flip.set(event_target_value(&ev).parse().unwrap_or(0))
                        >
                            <option value="0">"0"</option>
                            <option value="1">"1"</option>
                        </select>
                    </div>
                </div>

                <div class="grid grid-cols-3 gap-3 mt-3">
                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">"Turn 4"</label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                            prop:value=move || config_turn4.get()
                            on:input=move |ev| set_config_turn4.set(event_target_value(&ev))
                            placeholder="0"
                        />
                    </div>

                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">"Turn 5"</label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                            prop:value=move || config_turn5.get()
                            on:input=move |ev| set_config_turn5.set(event_target_value(&ev))
                            placeholder="0"
                        />
                    </div>

                    <div>
                        <label class="block text-[#888888] text-xs mb-1.5 font-medium">"Turn 6"</label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-sm text-white focus:border-[#00d9ff] focus:outline-none transition-colors font-mono"
                            prop:value=move || config_turn6.get()
                            on:input=move |ev| set_config_turn6.set(event_target_value(&ev))
                            placeholder="0"
                        />
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Wizard Footer with navigation buttons
#[component]
fn WizardFooter<V, S>(
    current_step: ReadSignal<WizardStep>,
    set_current_step: WriteSignal<WizardStep>,
    set_show_exit_warning: WriteSignal<bool>,
    validation_error: ReadSignal<Option<String>>,
    set_validation_error: WriteSignal<Option<String>>,
    validate_current_step: V,
    submit_robot: S,
    is_submitting: ReadSignal<bool>,
) -> impl IntoView
where
    V: Fn() -> Result<(), String> + Clone + Send + Sync + 'static,
    S: Fn() + Clone + Send + Sync + 'static,
{
    let can_go_back = move || current_step.get() != WizardStep::ConnectionDetails;
    let can_go_next = move || current_step.get() != WizardStep::AdditionalConfigurations;
    let is_last_required_step = move || current_step.get() == WizardStep::DefaultConfiguration;

    let go_back = move |_| {
        set_validation_error.set(None);
        let new_step = match current_step.get() {
            WizardStep::MotionDefaults => WizardStep::ConnectionDetails,
            WizardStep::JogDefaults => WizardStep::MotionDefaults,
            WizardStep::DefaultConfiguration => WizardStep::JogDefaults,
            WizardStep::AdditionalConfigurations => WizardStep::DefaultConfiguration,
            _ => current_step.get(),
        };
        set_current_step.set(new_step);
    };

    let validate_clone = validate_current_step.clone();
    let go_next = move |_| {
        set_validation_error.set(None);

        // Validate current step before proceeding
        if let Err(e) = validate_clone() {
            set_validation_error.set(Some(e));
            return;
        }

        let new_step = match current_step.get() {
            WizardStep::ConnectionDetails => WizardStep::MotionDefaults,
            WizardStep::MotionDefaults => WizardStep::JogDefaults,
            WizardStep::JogDefaults => WizardStep::DefaultConfiguration,
            WizardStep::DefaultConfiguration => WizardStep::AdditionalConfigurations,
            _ => current_step.get(),
        };
        set_current_step.set(new_step);
    };

    let submit_clone = submit_robot.clone();

    view! {
        <div class="border-t border-[#ffffff08] p-6">
            {move || validation_error.get().map(|err| view! {
                <div class="mb-4 p-3 bg-[#ef444410] border border-[#ef4444] rounded text-sm text-[#ef4444]">
                    {err}
                </div>
            })}

            <div class="flex items-center justify-between">
                <button
                    type="button"
                    class="px-4 py-2 text-sm text-[#888888] hover:text-white transition-colors"
                    on:click=move |_| set_show_exit_warning.set(true)
                >
                    "Cancel"
                </button>

                <div class="flex gap-3">
                    <Show when=can_go_back>
                        <button
                            type="button"
                            class="px-4 py-2 bg-[#1a1a1a] border border-[#ffffff08] rounded text-sm text-white hover:bg-[#222222] transition-colors"
                            on:click=go_back
                        >
                            "← Back"
                        </button>
                    </Show>

                    <Show when=can_go_next>
                        {
                            let go_next = go_next.clone();
                            move || view! {
                                <button
                                    type="button"
                                    class="px-4 py-2 bg-[#00d9ff] rounded text-sm text-black font-semibold hover:bg-[#00c4e6] transition-colors"
                                    on:click=go_next.clone()
                                >
                                    "Next →"
                                </button>
                            }
                        }
                    </Show>

                    <Show when=is_last_required_step>
                        {
                            let submit_clone = submit_clone.clone();
                            move || view! {
                                <button
                                    type="button"
                                    class=move || format!(
                                        "px-4 py-2 bg-[#22c55e] rounded text-sm text-black font-semibold transition-colors {}",
                                        if is_submitting.get() {
                                            "opacity-50 cursor-not-allowed"
                                        } else {
                                            "hover:bg-[#16a34a]"
                                        }
                                    )
                                    disabled=move || is_submitting.get()
                                    on:click={
                                        let submit_clone = submit_clone.clone();
                                        move |_| submit_clone()
                                    }
                                >
                                    {move || if is_submitting.get() { "Creating..." } else { "Create Robot" }}
                                </button>
                            }
                        }
                    </Show>
                </div>
            </div>
        </div>
    }
}

/// Exit Warning Modal
#[component]
fn ExitWarningModal<F1, F2>(
    on_cancel: F1,
    on_confirm: F2,
) -> impl IntoView
where
    F1: Fn() + Clone + Send + Sync + 'static,
    F2: Fn() + Clone + Send + Sync + 'static,
{
    let cancel = on_cancel.clone();
    let confirm = on_confirm.clone();

    view! {
        <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-[60]">
            <div class="bg-[#0d0d0d] border border-[#ffffff15] rounded-lg shadow-2xl w-full max-w-md p-6">
                <div class="flex items-start gap-3 mb-4">
                    <div class="w-10 h-10 rounded-full bg-[#ef444410] border border-[#ef4444] flex items-center justify-center flex-shrink-0">
                        <svg class="w-5 h-5 text-[#ef4444]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                        </svg>
                    </div>
                    <div class="flex-1">
                        <h3 class="text-base font-semibold text-white mb-1">"Discard Robot Creation?"</h3>
                        <p class="text-sm text-[#888888]">
                            "Your robot configuration will not be saved if you exit now. Are you sure you want to cancel?"
                        </p>
                    </div>
                </div>

                <div class="flex gap-3 justify-end">
                    <button
                        type="button"
                        class="px-4 py-2 bg-[#1a1a1a] border border-[#ffffff08] rounded text-sm text-white hover:bg-[#222222] transition-colors"
                        on:click=move |_| cancel()
                    >
                        "Continue Editing"
                    </button>
                    <button
                        type="button"
                        class="px-4 py-2 bg-[#ef4444] rounded text-sm text-white font-semibold hover:bg-[#dc2626] transition-colors"
                        on:click=move |_| confirm()
                    >
                        "Discard Changes"
                    </button>
                </div>
            </div>
        </div>
    }
}

