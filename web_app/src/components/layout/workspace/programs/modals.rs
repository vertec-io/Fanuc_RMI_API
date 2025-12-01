//! Program modals - New, Open, Save As, and CSV Upload modals.

use leptos::prelude::*;
use leptos::either::Either;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{FileReader, HtmlInputElement};
use crate::websocket::WebSocketManager;

/// New Program Modal - Simple modal to create a program with name and description
#[component]
pub fn NewProgramModal(
    on_close: impl Fn() + 'static + Clone + Send,
    on_created: impl Fn(i64) + 'static + Clone + Send,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    let (program_name, set_program_name) = signal("".to_string());
    let (description, set_description) = signal("".to_string());
    let (is_creating, set_is_creating) = signal(false);
    let (error_message, set_error_message) = signal::<Option<String>>(None);

    let on_close_clone = on_close.clone();
    let on_created_clone = on_created.clone();

    let programs = ws.programs;
    let api_error = ws.api_error;
    let initial_count = programs.get_untracked().len();

    Effect::new(move |_| {
        if is_creating.get() {
            if let Some(err) = api_error.get() {
                set_is_creating.set(false);
                let user_msg = if err.contains("UNIQUE constraint failed") {
                    "A program with this name already exists. Please choose a different name.".to_string()
                } else {
                    err
                };
                set_error_message.set(Some(user_msg));
                ws.clear_api_error();
                return;
            }

            let progs = programs.get();
            if progs.len() > initial_count {
                if let Some(newest) = progs.iter().max_by_key(|p| p.id) {
                    set_is_creating.set(false);
                    ws.set_message(format!("Program '{}' created", newest.name));
                    on_created_clone(newest.id);
                }
            }
        }
    });

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[400px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"/>
                        </svg>
                        "New Program"
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content
                <div class="p-3 space-y-3">
                    <Show when=move || error_message.get().is_some()>
                        <div class="bg-[#ff444420] border border-[#ff444440] rounded p-2 flex items-start gap-2">
                            <svg class="w-4 h-4 text-[#ff4444] flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            <span class="text-[10px] text-[#ff4444]">{move || error_message.get().unwrap_or_default()}</span>
                        </div>
                    </Show>
                    <div>
                        <label class="block text-[9px] text-[#888888] mb-1">"Program Name *"</label>
                        <input
                            type="text"
                            placeholder="e.g., Spiral Cylinder"
                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                            prop:value=move || program_name.get()
                            on:input=move |ev| {
                                set_program_name.set(event_target_value(&ev));
                                set_error_message.set(None);
                            }
                        />
                    </div>
                    <div>
                        <label class="block text-[9px] text-[#888888] mb-1">"Description"</label>
                        <textarea
                            placeholder="Optional description..."
                            rows="2"
                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none resize-none"
                            prop:value=move || description.get()
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                        ></textarea>
                    </div>
                    <p class="text-[8px] text-[#555555]">
                        "After creating the program, you can upload a CSV file with motion data."
                    </p>
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
                        on:click={
                            let on_close = on_close_clone.clone();
                            move |_| on_close()
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        class={move || format!(
                            "text-[10px] px-3 py-1.5 rounded {}",
                            if !program_name.get().is_empty() && !is_creating.get() {
                                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555] cursor-not-allowed"
                            }
                        )}
                        disabled=move || program_name.get().is_empty() || is_creating.get()
                        on:click=move |_| {
                            let name = program_name.get();
                            let desc = description.get();
                            let desc_opt = if desc.is_empty() { None } else { Some(desc) };
                            set_is_creating.set(true);
                            ws.create_program(name, desc_opt);
                            ws.list_programs();
                        }
                    >
                        {move || if is_creating.get() { "Creating..." } else { "Create Program" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Open Program Modal - Select a program to open
#[component]
pub fn OpenProgramModal(
    on_close: impl Fn() + 'static + Clone + Send,
    on_selected: impl Fn(i64) + 'static + Clone + Send,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let programs = ws.programs;

    Effect::new(move |_| {
        if ws.connected.get() {
            ws.list_programs();
        }
    });

    let on_close_clone = on_close.clone();

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[400px] max-h-[500px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                        </svg>
                        "Open Program"
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Program list
                <div class="flex-1 overflow-y-auto p-2">
                    {move || {
                        let progs = programs.get();
                        if progs.is_empty() {
                            Either::Left(view! {
                                <div class="text-[#555555] text-[10px] text-center py-8">
                                    "No programs available. Create one first."
                                </div>
                            })
                        } else {
                            Either::Right(progs.into_iter().map(|prog| {
                                let prog_id = prog.id;
                                let prog_name = prog.name.clone();
                                let lines_str = format!("{} lines", prog.instruction_count);
                                let on_selected = on_selected.clone();
                                view! {
                                    <button
                                        class="w-full text-left p-3 rounded border border-[#ffffff08] bg-[#0a0a0a] hover:border-[#00d9ff40] hover:bg-[#00d9ff10] transition-colors mb-1"
                                        on:click=move |_| on_selected(prog_id)
                                    >
                                        <div class="font-medium text-[11px] text-white mb-0.5">{prog_name}</div>
                                        <div class="text-[9px] text-[#555555]">{lines_str}</div>
                                    </button>
                                }
                            }).collect_view())
                        }
                    }}
                </div>

                // Footer
                <div class="flex justify-end p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
                        on:click=move |_| on_close_clone()
                    >
                        "Cancel"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Save As Modal - Save current program with a new name
#[component]
pub fn SaveAsProgramModal(
    on_close: impl Fn() + 'static + Clone + Send,
    on_saved: impl Fn(i64) + 'static + Clone + Send,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let current_program = ws.current_program;

    let (new_name, set_new_name) = signal("".to_string());
    let (new_description, set_new_description) = signal("".to_string());
    let (is_saving, set_is_saving) = signal(false);

    let on_close_clone = on_close.clone();
    let on_saved_clone = on_saved.clone();

    Effect::new(move |_| {
        if let Some(prog) = current_program.get() {
            set_new_name.set(format!("{} (copy)", prog.name));
            set_new_description.set(prog.description.unwrap_or_default());
        }
    });

    let programs = ws.programs;
    let initial_count = programs.get().len();

    Effect::new(move |_| {
        if is_saving.get() {
            let progs = programs.get();
            if progs.len() > initial_count {
                if let Some(newest) = progs.iter().max_by_key(|p| p.id) {
                    set_is_saving.set(false);
                    ws.set_message(format!("Program saved as '{}'", newest.name));
                    on_saved_clone(newest.id);
                }
            }
        }
    });

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[400px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"/>
                        </svg>
                        "Save As"
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content
                <div class="p-3 space-y-3">
                    <div>
                        <label class="block text-[9px] text-[#888888] mb-1">"New Program Name *"</label>
                        <input
                            type="text"
                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                            prop:value=move || new_name.get()
                            on:input=move |ev| set_new_name.set(event_target_value(&ev))
                        />
                    </div>
                    <div>
                        <label class="block text-[9px] text-[#888888] mb-1">"Description"</label>
                        <textarea
                            rows="2"
                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none resize-none"
                            prop:value=move || new_description.get()
                            on:input=move |ev| set_new_description.set(event_target_value(&ev))
                        ></textarea>
                    </div>
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
                        on:click={
                            let on_close = on_close_clone.clone();
                            move |_| on_close()
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        class={move || format!(
                            "text-[10px] px-3 py-1.5 rounded {}",
                            if !new_name.get().is_empty() && !is_saving.get() {
                                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555] cursor-not-allowed"
                            }
                        )}
                        disabled=move || new_name.get().is_empty() || is_saving.get()
                        on:click=move |_| {
                            let name = new_name.get();
                            let desc = new_description.get();
                            let desc_opt = if desc.is_empty() { None } else { Some(desc) };
                            set_is_saving.set(true);
                            ws.create_program(name, desc_opt);
                            ws.list_programs();
                        }
                    >
                        {move || if is_saving.get() { "Saving..." } else { "Save As" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// CSV Upload Modal - Upload CSV to an existing program
#[component]
pub fn CSVUploadModal(
    program_id: i64,
    on_close: impl Fn() + 'static + Clone + Send,
    on_uploaded: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    let (file_name, set_file_name) = signal::<Option<String>>(None);
    let (csv_content, set_csv_content) = signal::<Option<String>>(None);
    let (preview_lines, set_preview_lines) = signal::<Vec<String>>(vec![]);
    let (parse_error, set_parse_error) = signal::<Option<String>>(None);
    let (is_uploading, set_is_uploading) = signal(false);
    let (line_count, set_line_count) = signal(0usize);

    let on_close_clone = on_close.clone();

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[500px] max-h-[80vh] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
                        </svg>
                        "Upload CSV Data"
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content
                <div class="flex-1 overflow-y-auto p-3 space-y-3">
                    // File upload area
                    <label class="block border-2 border-dashed border-[#ffffff20] rounded-lg p-4 text-center hover:border-[#00d9ff40] transition-colors cursor-pointer">
                        <svg class="w-6 h-6 mx-auto mb-2 text-[#555555]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
                        </svg>
                        <p class="text-[10px] text-[#888888] mb-1">"Drop CSV file here or click to browse"</p>
                        <p class="text-[8px] text-[#555555]">"Required: x, y, z | Optional: w, p, r, speed, term_type"</p>
                        <input
                            type="file"
                            accept=".csv"
                            class="hidden"
                            on:change=move |ev| {
                                let target = ev.target().unwrap();
                                let input: HtmlInputElement = target.dyn_into().unwrap();
                                if let Some(files) = input.files() {
                                    if let Some(file) = files.get(0) {
                                        let name = file.name();
                                        set_file_name.set(Some(name.clone()));

                                        let reader = FileReader::new().unwrap();
                                        let reader_clone = reader.clone();
                                        let onload = Closure::wrap(Box::new(move || {
                                            if let Ok(result) = reader_clone.result() {
                                                if let Some(text) = result.as_string() {
                                                    let all_lines: Vec<&str> = text.lines().collect();
                                                    let data_lines = all_lines.len().saturating_sub(1);
                                                    set_line_count.set(data_lines);

                                                    let preview: Vec<String> = text.lines()
                                                        .skip(1)
                                                        .take(8)
                                                        .map(|s| s.to_string())
                                                        .collect();
                                                    set_preview_lines.set(preview);
                                                    set_csv_content.set(Some(text));
                                                    set_parse_error.set(None);
                                                }
                                            }
                                        }) as Box<dyn Fn()>);
                                        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                                        onload.forget();
                                        let _ = reader.read_as_text(&file);
                                    }
                                }
                            }
                        />
                    </label>

                    // File selected indicator
                    {move || file_name.get().map(|name| view! {
                        <div class="bg-[#0a0a0a] rounded p-2 flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <svg class="w-4 h-4 text-[#22c55e]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                </svg>
                                <span class="text-[10px] text-white">{name}</span>
                            </div>
                            <span class="text-[9px] text-[#666666]">{move || format!("{} lines", line_count.get())}</span>
                        </div>
                    })}

                    // Preview table
                    <Show when=move || !preview_lines.get().is_empty()>
                        <div>
                            <label class="block text-[9px] text-[#888888] mb-1">"Preview (first 8 lines)"</label>
                            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] overflow-hidden max-h-40 overflow-y-auto">
                                <table class="w-full text-[9px]">
                                    <thead class="sticky top-0 bg-[#0d0d0d]">
                                        <tr class="text-[#666666] border-b border-[#ffffff08]">
                                            <th class="text-left px-2 py-1">"#"</th>
                                            <th class="text-right px-2 py-1">"X"</th>
                                            <th class="text-right px-2 py-1">"Y"</th>
                                            <th class="text-right px-2 py-1">"Z"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {move || preview_lines.get().into_iter().enumerate().map(|(i, line)| {
                                            let parts: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();
                                            let x = parts.first().cloned().unwrap_or_else(|| "-".to_string());
                                            let y = parts.get(1).cloned().unwrap_or_else(|| "-".to_string());
                                            let z = parts.get(2).cloned().unwrap_or_else(|| "-".to_string());
                                            view! {
                                                <tr class="border-b border-[#ffffff05] text-[#cccccc]">
                                                    <td class="px-2 py-0.5 text-[#555555]">{i + 1}</td>
                                                    <td class="px-2 py-0.5 text-right font-mono">{x}</td>
                                                    <td class="px-2 py-0.5 text-right font-mono">{y}</td>
                                                    <td class="px-2 py-0.5 text-right font-mono">{z}</td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </Show>

                    // Error display
                    {move || parse_error.get().map(|err| view! {
                        <div class="bg-[#ff444410] border border-[#ff444420] rounded p-2 text-[9px] text-[#ff4444]">
                            {err}
                        </div>
                    })}
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
                        on:click={
                            let on_close = on_close_clone.clone();
                            move |_| on_close()
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        class={move || format!(
                            "text-[10px] px-3 py-1.5 rounded {}",
                            if csv_content.get().is_some() && !is_uploading.get() {
                                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555] cursor-not-allowed"
                            }
                        )}
                        disabled=move || csv_content.get().is_none() || is_uploading.get()
                        on:click={
                            let on_uploaded = on_uploaded.clone();
                            move |_| {
                                if let Some(content) = csv_content.get() {
                                    set_is_uploading.set(true);
                                    ws.upload_csv(program_id, content, None);
                                    on_uploaded();
                                }
                            }
                        }
                    >
                        {move || if is_uploading.get() { "Uploading..." } else { "Upload CSV" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

