use leptos::prelude::*;
use leptos::either::*;

/// Formatta un errore di compilazione Typst per renderlo più leggibile
fn format_typst_error(error: &str) -> String {
    // Estrae il messaggio principale dall'errore di debug
    if let Some(start) = error.find("message: \"") {
        let after_message = &error[start + 10..];
        if let Some(end) = after_message.find("\", trace:") {
            return after_message[..end].to_string();
        }
    }

    // Fallback: ritorna l'errore originale
    error.to_string()
}

#[component]
pub fn Preview(
    output: ReadSignal<String>,
    error: ReadSignal<Option<String>>,
) -> impl IntoView {
    view! {
        // Struttura semplificata: flex column con header fisso e area scrollabile
        <div class="flex flex-col h-full">
            // Header fisso
            <div class="flex items-center gap-2 px-4 py-3 bg-base-200 border-b border-base-300 flex-shrink-0">
                <span class="icon-[lucide--eye] text-lg text-accent"></span>
                <h2 class="text-sm font-semibold uppercase tracking-wide text-base-content/70">"Preview"</h2>
            </div>

            // Area scrollabile con height: 0 e flex: 1 per forzare contenimento
            <div class="preview-scroll-area bg-base-100">
                {move || {
                    if let Some(err) = error.get() {
                        let formatted_error = format_typst_error(&err);
                        Either::Left(view! {
                            <div class="alert alert-error m-4 shadow-lg">
                                <span class="icon-[lucide--alert-circle] text-2xl"></span>
                                <div>
                                    <h3 class="font-bold">"Compilation Error"</h3>
                                    <div class="text-sm">{formatted_error}</div>
                                </div>
                            </div>
                        })
                    } else {
                        // Contenuto SVG reale da Typst
                        Either::Right(view! {
                            <div class="preview-content" inner_html=move || output.get()></div>
                        })
                    }
                }}
            </div>
        </div>
    }
}
