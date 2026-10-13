//! The words the library says by itself: hints, default actions, the names
//! of things in a picker. A host sets them once, in its own language.

use std::sync::{PoisonError, RwLock};

/// Every text a component shows that its host did not give it. They are
/// English unless the host says otherwise:
///
/// ```
/// limn::set_labels(limn::Labels::SPANISH);
/// // Or a few of them, in any language:
/// limn::set_labels(limn::Labels { close: "Fermer", cancel: "Annuler", ..limn::Labels::ENGLISH });
/// # limn::set_labels(limn::Labels::ENGLISH);
/// ```
///
/// A component that takes a label of its own, such as an alert dialog's
/// actions, uses these only when it is given none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Labels {
    /// The hint of the button that closes a modal.
    pub close: &'static str,
    /// The hint of the button that dismisses an alert.
    pub dismiss_notice: &'static str,
    /// An alert dialog's confirming action, unless it is given one.
    pub confirm: &'static str,
    /// An alert dialog's cancelling action, unless it is given one.
    pub cancel: &'static str,
    /// Before the name of an attachment, on the hint of its remove button.
    pub remove: &'static str,
    /// The hint of the search action of a path.
    pub search: &'static str,
    /// The title of the hint that shows a shortened path whole.
    pub full_path: &'static str,
    /// The hint of the button that closes an open collapsible from its foot.
    pub collapse: &'static str,
    /// A coach mark: the action that goes on.
    pub next: &'static str,
    /// A coach mark: the action of the last step.
    pub done: &'static str,
    /// A coach mark: the action that goes back.
    pub back: &'static str,
    /// Between a position and a total: "2 of 5".
    pub of: &'static str,
    /// The text context menu.
    pub cut: &'static str,
    /// The text context menu.
    pub copy: &'static str,
    /// The text context menu.
    pub paste: &'static str,
    /// The text context menu.
    pub select_all: &'static str,
    /// Over the date of a date and time picker.
    pub date: &'static str,
    /// Over the time of a date and time picker.
    pub local_time: &'static str,
    /// A time picker.
    pub hour: &'static str,
    /// A time picker.
    pub minute: &'static str,
    /// The image viewer.
    pub zoom_out: &'static str,
    /// The image viewer.
    pub fit_to_window: &'static str,
    /// The image viewer.
    pub zoom_in: &'static str,
    /// The image viewer.
    pub close_image: &'static str,
    /// The image viewer, while its image has not arrived.
    pub loading_image: &'static str,
    /// The image viewer.
    pub previous_image: &'static str,
    /// The image viewer.
    pub next_image: &'static str,
    /// In place of a picture that could not be drawn.
    pub preview_unavailable: &'static str,
    /// A code review.
    pub previous_change: &'static str,
    /// A code review.
    pub next_change: &'static str,
    /// A code review: discard the change.
    pub undo: &'static str,
    /// A code review: accept the change.
    pub keep: &'static str,
    /// A code review whose host gave no reason.
    pub review_unavailable: &'static str,
    /// The code editor: a folded block.
    pub expand_block: &'static str,
    /// The code editor: an open block.
    pub fold_block: &'static str,
    /// Before a range of lines: "Lines 4–9".
    pub lines: &'static str,
    /// Before one line: "Line 12".
    pub line: &'static str,
    /// The code editor: the line pinned over a block.
    pub block_start: &'static str,
    /// The code editor.
    pub breakpoint_verified: &'static str,
    /// The code editor.
    pub breakpoint_verified_detail: &'static str,
    /// The code editor.
    pub breakpoint_rejected: &'static str,
    /// The code editor.
    pub breakpoint_rejected_detail: &'static str,
    /// The code editor.
    pub breakpoint_disabled: &'static str,
    /// The code editor.
    pub breakpoint_disabled_detail: &'static str,
    /// The code editor.
    pub breakpoint_pending: &'static str,
    /// The code editor.
    pub breakpoint_pending_detail: &'static str,
    /// The code editor.
    pub breakpoint_local_detail: &'static str,
    /// The code editor.
    pub breakpoint_add: &'static str,
    /// The code editor.
    pub breakpoint_add_detail: &'static str,
    /// A calendar's weekdays, Monday first, two letters each.
    pub weekdays: [&'static str; 7],
    /// A calendar's months, January first.
    pub months: [&'static str; 12],
    /// The code editor's status line: a file in no language.
    pub plain_text: &'static str,
    /// The code editor's status line.
    pub read_only: &'static str,
    /// The code editor's status line: changes not yet saved.
    pub unsaved: &'static str,
    /// The code editor's status line.
    pub saved: &'static str,
}

impl Labels {
    /// The library's own words in English; the default.
    pub const ENGLISH: Self = Self {
        close: "Close",
        dismiss_notice: "Dismiss",
        confirm: "Continue",
        cancel: "Cancel",
        remove: "Remove",
        search: "Search",
        full_path: "Full path",
        collapse: "Collapse",
        next: "Next",
        done: "Got it",
        back: "Back",
        of: "of",
        cut: "Cut",
        copy: "Copy",
        paste: "Paste",
        select_all: "Select all",
        date: "Date",
        local_time: "Local time",
        hour: "Hour",
        minute: "Minute",
        zoom_out: "Zoom out",
        fit_to_window: "Fit to window",
        zoom_in: "Zoom in",
        close_image: "Close image",
        loading_image: "Loading image…",
        previous_image: "Previous image",
        next_image: "Next image",
        preview_unavailable: "Preview unavailable",
        previous_change: "Previous change",
        next_change: "Next change",
        undo: "Undo",
        keep: "Keep",
        review_unavailable: "Review is not available for this version.",
        expand_block: "Expand block",
        fold_block: "Fold block",
        lines: "Lines",
        line: "Line",
        block_start: "Back to the start of the block",
        breakpoint_verified: "Breakpoint verified",
        breakpoint_verified_detail: "The adapter confirmed this breakpoint.",
        breakpoint_rejected: "Breakpoint rejected",
        breakpoint_rejected_detail: "The adapter could not verify this breakpoint. Check the session's message.",
        breakpoint_disabled: "Breakpoint disabled",
        breakpoint_disabled_detail: "This breakpoint is disabled and does not stop execution.",
        breakpoint_pending: "Breakpoint pending",
        breakpoint_pending_detail: "The adapter has not confirmed this breakpoint yet.",
        breakpoint_local_detail: "A local mark: no debugger is connected. It does not stop execution and is cleared when the file is edited.",
        breakpoint_add: "Set a breakpoint",
        breakpoint_add_detail: "Toggle a breakpoint on this line.",
        weekdays: ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"],
        months: [
            "January", "February", "March", "April", "May", "June", "July", "August", "September", "October",
            "November", "December",
        ],
        plain_text: "Text",
        read_only: "Read only",
        unsaved: "Unsaved",
        saved: "Saved",
    };

    /// The library's own words in Spanish.
    pub const SPANISH: Self = Self {
        close: "Cerrar",
        dismiss_notice: "Cerrar aviso",
        confirm: "Continuar",
        cancel: "Cancelar",
        remove: "Quitar",
        search: "Buscar",
        full_path: "Ruta completa",
        collapse: "Contraer",
        next: "Siguiente",
        done: "Entendido",
        back: "Atrás",
        of: "de",
        cut: "Cortar",
        copy: "Copiar",
        paste: "Pegar",
        select_all: "Seleccionar todo",
        date: "Fecha",
        local_time: "Hora local",
        hour: "Hora",
        minute: "Minuto",
        zoom_out: "Alejar",
        fit_to_window: "Ajustar a la ventana",
        zoom_in: "Acercar",
        close_image: "Cerrar imagen",
        loading_image: "Cargando imagen…",
        previous_image: "Imagen anterior",
        next_image: "Imagen siguiente",
        preview_unavailable: "Vista previa no disponible",
        previous_change: "Cambio anterior",
        next_change: "Cambio siguiente",
        undo: "Deshacer",
        keep: "Conservar",
        review_unavailable: "La revisión no está disponible para esta versión.",
        expand_block: "Expandir bloque",
        fold_block: "Plegar bloque",
        lines: "Líneas",
        line: "Línea",
        block_start: "Volver al inicio del bloque",
        breakpoint_verified: "Punto de interrupción verificado",
        breakpoint_verified_detail: "El adaptador confirmó este punto de interrupción.",
        breakpoint_rejected: "Punto de interrupción rechazado",
        breakpoint_rejected_detail: "El adaptador no pudo verificar este punto. Revisa el mensaje de la sesión.",
        breakpoint_disabled: "Punto de interrupción desactivado",
        breakpoint_disabled_detail: "Este punto está desactivado y no detiene la ejecución.",
        breakpoint_pending: "Punto de interrupción pendiente",
        breakpoint_pending_detail: "El adaptador todavía no confirmó este punto de interrupción.",
        breakpoint_local_detail: "Marca local · sin depurador conectado. No detiene la ejecución. Se borra al editar el archivo.",
        breakpoint_add: "Marcar punto de interrupción",
        breakpoint_add_detail: "Alternar punto de interrupción en esta línea.",
        weekdays: ["Lu", "Ma", "Mi", "Ju", "Vi", "Sa", "Do"],
        months: [
            "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "octubre",
            "noviembre", "diciembre",
        ],
        plain_text: "Texto",
        read_only: "Solo lectura",
        unsaved: "Sin guardar",
        saved: "Guardado",
    };
}

impl Default for Labels {
    fn default() -> Self {
        Self::ENGLISH
    }
}

static CURRENT: RwLock<Labels> = RwLock::new(Labels::ENGLISH);

/// The labels in force.
pub fn labels() -> Labels {
    *CURRENT.read().unwrap_or_else(PoisonError::into_inner)
}

/// Replace the labels for the whole process. Elements built from here on use
/// them, so an application that rebuilds its tree every frame changes
/// language on the next one.
pub fn set_labels(labels: Labels) {
    *CURRENT.write().unwrap_or_else(PoisonError::into_inner) = labels;
}
