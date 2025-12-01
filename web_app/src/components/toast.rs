//! Toast notification component for displaying API messages.

use leptos::prelude::*;
use crate::websocket::WebSocketManager;

/// Toast notification container - displays API messages and errors.
#[component]
pub fn ToastContainer() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let api_message = ws.api_message;
    let api_error = ws.api_error;

    view! {
        // Position: bottom-left to avoid overlapping with jog controls (bottom-right)
        <div class="fixed bottom-4 left-4 z-50 flex flex-col gap-2 max-w-sm">
            // Success/Info toast
            <Show when=move || api_message.get().is_some()>
                <SuccessToast message=Signal::derive(move || api_message.get().unwrap_or_default()) />
            </Show>

            // Error toast
            <Show when=move || api_error.get().is_some()>
                <ErrorToast message=Signal::derive(move || api_error.get().unwrap_or_default()) />
            </Show>
        </div>
    }
}

/// Success toast notification
#[component]
fn SuccessToast(message: Signal<String>) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let (visible, set_visible) = signal(true);

    // Auto-dismiss after 5 seconds
    Effect::new(move |_| {
        if visible.get() {
            set_timeout(
                move || {
                    set_visible.set(false);
                    ws.clear_api_message();
                },
                std::time::Duration::from_secs(5),
            );
        }
    });

    view! {
        <Show when=move || visible.get()>
            <div class="flex items-start gap-2 p-3 rounded-lg border shadow-lg bg-[#0d0d0d] border-[#22c55e40]">
                <div class="text-[#22c55e]">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
                    </svg>
                </div>
                <p class="flex-1 text-[11px] text-white">{move || message.get()}</p>
                <button
                    class="text-[#666666] hover:text-white transition-colors"
                    on:click=move |_| {
                        set_visible.set(false);
                        ws.clear_api_message();
                    }
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                </button>
            </div>
        </Show>
    }
}

/// Error toast notification
#[component]
fn ErrorToast(message: Signal<String>) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let (visible, set_visible) = signal(true);

    // Auto-dismiss after 8 seconds (longer for errors)
    Effect::new(move |_| {
        if visible.get() {
            set_timeout(
                move || {
                    set_visible.set(false);
                    ws.clear_api_error();
                },
                std::time::Duration::from_secs(8),
            );
        }
    });

    view! {
        <Show when=move || visible.get()>
            <div class="flex items-start gap-2 p-3 rounded-lg border shadow-lg bg-[#0d0d0d] border-[#ff444440]">
                <div class="text-[#ff4444]">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                </div>
                <p class="flex-1 text-[11px] text-white">{move || message.get()}</p>
                <button
                    class="text-[#666666] hover:text-white transition-colors"
                    on:click=move |_| {
                        set_visible.set(false);
                        ws.clear_api_error();
                    }
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                </button>
            </div>
        </Show>
    }
}
