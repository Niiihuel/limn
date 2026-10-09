# lienzo

Motor de interfaz retenido y neutral al backend. La aplicación construye un árbol de
`Element`, `UiTree` calcula el layout y conserva el estado de controles por ID,
y el resultado es un `Frame` con comandos de dibujo y regiones interactivas.
El backend de ventana traduce esos comandos a su propio renderer.

## Contrato para el backend

1. Construir el árbol con `Element::{column,row,panel,overlay,popover,scroll,text,formatted_text,icon,button,icon_button,text_input,text_area,code_editor,code_diff,skeleton,spinner,spacer}`.
2. Conservar una instancia de `UiTree` y llamar `set_root` al actualizar el árbol.
   Los controles con el mismo ID conservan foco, cursor, selección y scroll.
3. Llamar `layout(Size)` antes de entregar eventos de puntero y otra vez antes
   de pintar. Los eventos producen `UiAction` para botones y entradas de texto.
4. Dibujar `Frame::commands` en orden. `PushClip` y `PopClip` delimitan el recorte
   de scroll e input; el renderer debe intersectar clips anidados.

`FontMetrics` y la fuente elegida por el backend deben coincidir. El tamaño
predeterminado es 14 px lógicos y la familia predeterminada es monoespaciada.
`Element::font_family(FontFamily::Sans)` elige la fuente de UI;
`Element::font_size(...)` ajusta su tamaño. El host calibra la fuente mono con
`set_metrics` y la fuente sans con `set_sans_metrics`, ambas a 14 px. Para
ajustar líneas con las mismas métricas que el renderer, el host registra
`UiTree::set_text_measurer(|texto, tamaño, familia| ancho)`. Si no lo hace,
el motor usa las métricas calibradas como alternativa. `formatted_text`
recibe `TextSpan` con color, tamaño, familia y fondo opcional por segmento;
sus saltos se calculan con el medidor del host y respetan los grafemas Unicode.
Los segmentos se pintan como comandos `Text` normales y pueden seleccionarse
o copiarse con Ctrl+C. `TextSpan::action(id)` añade una acción inline: los
fragmentos envueltos usan sus áreas de texto reales y comparten un destino de
Tab, activable con Enter o Espacio. `text_input` es de una
línea; `text_area` permite varias, recorta y desplaza su contenido con la
rueda. El backend aplica el factor de escala de pantalla.

`Element::border(color, width)` crea un trazo interno; `radius` se aplica tanto
al fondo como al borde. El trazo no modifica el tamaño calculado del elemento.

`Element::icon(Icon::...)` crea un icono semántico y
`Element::icon_button(id, Icon::..., label)` crea un botón interactivo con icono.
Una etiqueta vacía produce un botón de sólo icono. Todos los iconos ocupan
16 px lógicos; `icon_size` se conserva temporalmente por compatibilidad y no
altera ese tamaño. `foreground(color)` aplica una sola tinta semántica.
El motor emite `DrawCommand::Icon`; el backend elige el arte. En Agents Window,
el backend usa un conjunto pequeño de Solar Icons Linear incluido en
`apps/agents-window/assets/solar/`, con sus licencias y atribución.

Para recursos del proyecto, `file_icon("src/main.rs")`,
`folder_icon("src", expanded)` y `language_icon("rust")` comparten el catálogo
Material Icon Theme 5.38.1: nombres exactos, extensiones compuestas, aliases y
carpetas abiertas/cerradas. Devuelven `Icon::FileType(FileIcon)`; el host toma
el SVG embebido con `FileIcon::svg()`, conserva sus colores y aplica sólo el alfa
de `foreground`. Los glyphs se cachean por tamaño físico; resolver un path no
lee el disco ni instala dependencias en runtime. La licencia MIT, el pin, los
hashes y el script reproducible se documentan en
[assets/material-icon-theme/NOTICE.md](assets/material-icon-theme/NOTICE.md).

`Avatar::new(nombre, palette).image(foto).size(32.0)` muestra una foto circular
con `RasterImage`, o la inicial del nombre cuando no hay foto. El host obtiene
la imagen y la recorta a un cuadrado fuera del hilo visual; el componente no
hace solicitudes de red. `AlertDialog::confirm_enabled(false)` conserva una
acción pasiva mientras una operación asíncrona prepara su contenido.

La navegación con `Key::Tab` y Shift+Tab recorre botones e inputs en orden de
árbol y conserva el foco al reconstruir controles con los mismos IDs. Enter y
Space activan un botón enfocado; en un input, Enter emite `Submitted` y los
espacios llegan como `UiEvent::Text(" ")`.
Un `CodeEditor` enfocado procesa Tab/Shift+Tab como indentación; sus atajos y
selección pertenecen al documento, no al estado de los inputs de formulario.

Cada control montado debe tener un ID visual distinto, aunque comparta una
acción con otro. `Element::dispatch_as("acción")` permite emitir la misma
acción desde IDs diferentes sin compartir hover, transición de color o foco.
El ID conserva el estado del mismo control entre llamadas a `set_root`.

En `text_area`, Enter inserta un salto de línea y Ctrl+Enter emite `Submitted`.
Shift+flechas selecciona texto, Ctrl+A selecciona todo, y las flechas arriba y
abajo navegan por las líneas visibles. `input_value` y `set_input_value`
funcionan igual para ambos tipos de editor.

Los botones interpolan sus fondos de hover y pulsación durante 120 ms por
defecto. `transition_ms(0)` fuerza una respuesta inmediata;
`UiTree::set_reduced_motion(true)` desactiva las transiciones. El host debe
pedir un repintado corto mientras `UiTree::is_animating()` sea verdadero.
`ScalarTransition` anima valores escalares (ancho, posición u opacidad) con
`retarget(value, now, duration)` sin saltos a mitad del movimiento;
`retarget_with_motion(..., reduced_motion)` y `snap_to(...)` resuelven cambios
instantáneos. `Easing::OutCubic` y `Easing::InOutCubic` ofrecen curvas breves y
suaves. La aplicación conserva esta transición entre reconstrucciones y usa
`is_active(now)` para programar repintados.

`Tabs` mueve el fondo de selección entre controles con el mismo `Tabs::id()`;
si no se indica un ID, utiliza el primer ID de acción. `animation_ms(0)` hace
el cambio instantáneo. `Popover` aplica una entrada de 140 ms y una salida de
hasta 110 ms a la pintura (opacidad y desplazamiento de 4 px). Al cerrar,
retira de inmediato las regiones de clic y foco; únicamente persiste la pintura
durante la salida. Si se reabre mientras sale, continúa desde la opacidad
visible. `Popover::animation_ms(0)` desactiva ambos movimientos.
Ambos respetan `UiTree::set_reduced_motion(true)`.

`Element::horizontal_scroll(id, child)` mantiene un viewport horizontal con
recorte de pintura y acciones. El hijo conserva su ancho completo; la rueda
sobre el viewport, Shift+rueda y los deltas horizontales lo desplazan.
`.reveal_horizontal_item(id)` revela el descendiente seleccionado al cambiar
esa selección o el ancho. Reconstruir el árbol con el mismo ID conserva el
scroll manual. Los controles que deban permanecer fijos van fuera del viewport.

`Element::horizontal_scroll_2d(id, child)` conserva esa geometría y usa únicamente
deltas horizontales o Shift+rueda para mover X. La rueda vertical queda disponible
para el `ScrollArea` exterior; un gesto diagonal puede mover ambos ejes. Es la
variante para documentos ampliados, mientras que las tiras de pestañas conservan
`horizontal_scroll`.

`Element::faded_text(título, ancho_del_fade)` mantiene una sola línea y desvanece
su extremo derecho sólo cuando el texto no cabe. Conserva el texto original y
su forma Unicode; usa clips disjuntos y alfa estático en comandos `Text`, sin
elipsis ni una máscara del color de fondo. No agrega acciones, selección ni
repintados de animación. Un título que cabe se pinta una sola vez.

`Breadcrumb::new(id, items, palette).width(ancho).height(28.0)` presenta una
ruta compacta con separadores `ChevronRight`. Cada `BreadcrumbItem::new(label)`
puede recibir `.action(id)`, `.icon(icon)` y `.tooltip(ruta_completa)`.
El último elemento representa el recurso actual. Cada segmento sólo recibe
clic y teclado si el host suministra una acción, incluso el archivo actual.
Los segmentos conservan su ancho natural, con separadores de 10 px y espacios
de 4 px. El espacio sobrante queda fuera del hover y del área clicable; una
ranura acotada conserva el recorte al estrechar el panel.
El ancho prioriza el recurso actual y conserva la raíz cuando hay espacio.
Los ancestros intermedios se resumen; si tienen acciones, `.overflow_open(true)`
abre su menú controlado. El host procesa `{id}:overflow_toggle` y
`{id}:overflow_dismiss`. Los ancestros pasivos siguen disponibles en la ruta
completa del tooltip, sin un botón inerte. Su descripción envuelve rutas largas.
La API no accede al disco ni infiere destinos de navegación.

```rust
use lienzo::{Breadcrumb, BreadcrumbItem, Component, ComponentPalette};

let path = Breadcrumb::new("review-path", vec![
    BreadcrumbItem::new("Cambios").action("review_detail_back"),
    BreadcrumbItem::new("src"),
    BreadcrumbItem::new("main.rs").tooltip("src/main.rs"),
], ComponentPalette::neutral_dark()).width(280.0).render();
```

La composición se estudió en [Rust/UI Breadcrumb](https://rust-ui.com/docs/components/breadcrumb).
Es una implementación nativa propia; no requiere Leptos, HTML ni Tailwind.

`Element::clip_children()` recorta pintura y regiones interactivas de los
descendientes al contenido del contenedor. El fondo, borde y sombra del
contenedor se dibujan completos. El clip es rectangular y permite contraer
un dock sin que su contenido ancho invada el panel vecino.
Aplicado a `Element::overlay`, mantiene también la capa superpuesta en el
recorte y orden de pintura locales; sirve para botones sobre una miniatura.
`UiTree::focus(id)` enfoca un control existente después de `layout`, útil para
abrir un buscador listo para escribir.

`input_cursor`, `input_selection` y `set_input_cursor` exponen posiciones en
caracteres Unicode para sugerencias bajo el cursor. `Element::popover_non_modal()`
monta un popup sin scope de foco ni backdrop bloqueante: el host administra sus
atajos y cierre mientras el editor conserva el foco. Los popovers ordinarios
conservan su navegación y captura de foco.

`PermissionSelector` presenta filas compactas con icono y selección;
las explicaciones de cada opción se muestran en tooltips laterales.
`PermissionOption` define acción, etiqueta, descripción y disponibilidad.
`.separator_before()` permite separar grupos de opciones dentro del mismo menú.
El host proporciona las políticas reales, conserva la selección y procesa
las acciones; el componente no modifica permisos ni aislamiento por sí mismo.

`SelectorSizing::COMPACT` y `CATALOG` comparten las medidas de filas, iconos,
tipografía y scroll. Definen anchos preferidos de 256 y 320 px, filas de
32 px (44 con un ID secundario) y tooltips de hasta 320 px.
`DropdownMenu::sizing(...)` adopta el perfil; `fit_width(...)` lo adapta al
espacio del host. Ver [Selectores y popovers](../../docs/ui-selector-standard.md).

El motor no depende de Warp ni de sus recursos. Toma como referencia la
separación conceptual entre estado, árbol visual, layout, escena y renderer que
Warp documenta para su framework de UI.

`Skeleton` representa contenido en carga sin agregar un objetivo de clic. Su
pulso de color se calcula en `motion.rs`, conserva la fase por ID entre cambios
del árbol y no cambia la geometría. El host sigue usando `is_animating()` para
pedir repintados; al quitar los Skeletons del árbol, deja de pedirlos. Con
`set_reduced_motion(true)` el bloque queda estático.

`Marker::new(id, label, palette)` presenta un hecho o una actividad de la
conversación con las variantes `Default`, `Separator` y `Border`. `icon` añade
un icono pasivo; `loading(true)` añade un Spinner y un barrido de luz sobre el
texto, y `shimmer(true)` activa sólo ese barrido. Por defecto no recibe clics
ni foco. `action(action_id)` añade una acción explícita de puntero y teclado.
El llamador determina el estado real, la etiqueta y cuándo termina la carga;
el componente no interpreta herramientas, resultados ni ramas de Git.
`.thinking(true)` muestra un cerebro estático y un shimmer suave del texto,
sin spinner adicional; con movimiento reducido queda estático.
`.truncate(true)` mantiene la etiqueta en una sola línea mediante recorte
medido y elipsis, tanto para texto normal como para shimmer. Es optativo: las
etiquetas existentes siguen envolviendo su contenido. No cambia la altura
mínima de la fila/acción; el host puede añadir un Tooltip con el rótulo completo.

`TextShimmer` también puede usarse sin Marker. Su texto se mide, envuelve y
selecciona con el mismo contrato del texto normal; mantiene una única fuente
para Ctrl+C y no cambia el layout. La fase se conserva por ID entre
reconstrucciones. El barrido usa comandos `Text` con clips disjuntos, por lo
que no requiere un shader o renderer nuevo. Sólo las instancias visibles
mantienen el repintado; con movimiento reducido se dibuja texto estático.
Las pruebas usan reloj fijo, medición proporcional, selección Unicode, cambio
de árbol, clipping, variantes y acciones a varios anchos.
El patrón visual se consultó en [Rust/UI Marker](https://rust-ui.com/docs/components/marker);
esta implementación pertenece al motor nativo y no requiere Dioxus ni Tailwind.

`Collapsible::new(toggle_id, title, content, palette)` muestra un header compacto
de 32 px con chevron. `expanded(bool)` monta el cuerpo indentado sólo cuando se
abre; `icon`, `loading` y `detail` añaden icono/spinner y metadata breve. El host
conserva el estado de expansión y responde a `UiAction::Clicked` con el ID dado.
El componente no interpreta ni ejecuta el contenido; sirve para razonamiento
público, argumentos provisionales y grupos de actividad. Los IDs estables
conservan foco y fase del spinner durante el streaming. El host puede construir
un `Element::spacer()` para el cuerpo cerrado y evitar trabajo de presentación.

`ToolActivity::new(toggle_id, title, content, palette)` presenta una operación
con encabezado de 32 px, icono y destino con elipsis medida. `command(true)` usa
tipografía monoespaciada para el destino; `running(true)` anima el título y
respeta movimiento reducido. `note(...)` conserva errores o permisos visibles
con el cuerpo cerrado. El host controla `expanded(bool)`, conserva un ID estable
entre preview y ejecución y suministra únicamente estados observados. La salida
de procesos se compone como contenido pasivo; el componente no envía stdin ni
ejecuta herramientas al abrirse.

```rust
use lienzo::{Component, ComponentPalette, Skeleton};

let placeholder = Skeleton::new("catalog-title", ComponentPalette::neutral_dark())
    .width(180.0)
    .height(12.0)
    .radius(6.0)
    .render();
```

`Spinner` indica una operación sin duración conocida. Su forma predeterminada
es un aro incompleto que gira continuamente; `SpinnerVariant::Spokes` ofrece la
variante de radios. El backend dibuja ambas con trazos vectoriales. Conserva la
fase por ID, no crea una región interactiva y se detiene en una posición fija
con `UiTree::set_reduced_motion(true)`.

`Button::loading(true)` e `IconButton::loading(true)` usan este spinner circular
en el espacio del icono principal. Mantienen la etiqueta y deshabilitan la
activación mientras la operación está pendiente. El host conserva el estado
de la operación; al terminar, `.loading(false)` devuelve el botón normal.

```rust
use lienzo::{Color, Component, Spinner, SpinnerVariant};

let progress = Spinner::new("search-progress", Color::WHITE)
    .size(28.0)
    .variant(SpinnerVariant::Circle)
    .render();
```

`Modal` sirve para formularios que deben quedar centrados sobre la ventana. Su
`body` es desplazable cuando se fija `height`, con la barra reservada fuera del
ancho del formulario; la cabecera y el pie permanecen visibles.
`initial_focus` elige el primer campo; Escape y la X gruesa sin círculo
despachan el mismo ID de cierre, y el foco vuelve al control anterior al cerrar.
El fondo queda bloqueado y `backdrop_dismiss(true)` permite cerrar al pulsarlo
cuando el flujo admite descartarlo.

`Stepper` muestra pasos completados, activo y pendientes en una barra compacta.
Recibe el paso actual del host y emite los IDs de los pasos ya alcanzados;
los pendientes no se pueden pulsar para saltar validaciones. El host muestra
un solo formulario por paso, conserva los valores al volver y decide si
`Siguiente` puede avanzar. Automatizaciones lo usa para Contenido, Horario y
Agente dentro del `Modal`, con un único scroll exterior al formulario.

`DatePicker` ofrece entrada ISO y calendario mensual navegable; `TimePicker`
ofrece entrada `HH:MM` y ajustes de hora/minuto. `DateTimePicker` los combina
sin unir sus estados, para conservar lo escrito en cada campo mientras se
edita el otro. El host procesa los IDs de navegación y selección que documenta
cada componente y valida los valores antes de guardar.

## Controles compartidos

`HeatmapChart`, `BarChart` y `ChartLegend` permiten crear gráficos nativos con
el mismo árbol de elementos y la paleta del host. El heatmap acepta columnas
con celdas de intensidad 0–4 y huecos para observaciones ausentes; las barras
muestran etiquetas y valores sobre una escala común. `.width(...)` adapta
su geometría al espacio disponible. Los tooltips de datos usan regiones de
hover sin acciones de clic ni paradas de Tab. Consulta
[Charts nativos](../../docs/charts.md) para ejemplos e integración.

`Button` es la acción compartida de la aplicación. `ButtonContent` distingue
etiqueta, icono y ambos; `IconButton` conserva la API breve para acciones de
icono. `ControlSize::button_sizing()` centraliza alto, tipografía, espaciado y
radio. El tema llega mediante `ComponentPalette` y `ButtonVariant`; `disabled`
quita la región de clic y foco. `.tooltip(...)` y
`.tooltip_shortcut(...)` asocian una ayuda al mismo ID de acción. El atajo se
muestra en la ayuda; la activación por teclado sigue el enrutamiento de
`UiTree` y la aplicación interpreta `UiAction::Clicked`.

`Empty` ofrece un estado vacío centrado con icono, título, descripción y una o
varias acciones `Button`. Mantiene una altura legible aun si el contenedor pide
menos espacio: 256 px, o 228 px con `.compact()`. `.height(...)` permite darle
más espacio, por ejemplo para centrarlo en el viewport. `.action_element(...)`
admite un control propio en la misma fila centrada. Por ejemplo:

```rust
use lienzo::{Button, ButtonVariant, Component, ComponentPalette, Empty, Icon};

let palette = ComponentPalette::neutral_dark();
let retry = Button::new("retry", "Reintentar", palette)
    .variant(ButtonVariant::Secondary)
    .render();
let empty = Empty::new("Sin resultados", "Prueba otro término.", palette)
    .icon(Icon::Search)
    .compact()
    .height(320.0)
    .action_element(retry)
    .render();
```

El borde discontinuo y la versión compacta sirven para páginas y paneles.
`Kbd::new("Ctrl+Shift+M", palette)` muestra el atajo
completo con una cápsula por tecla, siguiendo el estilo de GNOME/Adwaita:
fuente Sans, esquinas de 6 px, superficie translúcida y relieve inferior.
`Ctrl++` conserva la tecla literal `+`; `↑↓` muestra ambas flechas. Los grupos
se miden con la fuente real y `.max_width(...)` omite el atajo completo si no
cabe, sin abreviar su significado. Un atajo vacío no dibuja una cápsula.
Úsalo también en filas y listas; `Tooltip::shortcut`,
`Search::shortcut`, `DropdownMenuItem::shortcut` y `MenuItem::shortcut` aplican
el mismo tratamiento. Para un estado como “Abierta”, usa
`DropdownMenuItem::annotation` y conserva el texto sin apariencia de tecla.
`Search` (también disponible como `SearchField`) permite agregar una pista con
`.shortcut("Ctrl+K")` dentro del único objetivo de foco del campo. Todas las
instancias de `Tabs` comparten el carril redondeado del selector de
Grupo/Proyecto, incluido su indicador animado.

`TextField` y `TextArea` aplican la misma paleta, borde de foco y densidad a los
formularios. Ambos conservan el valor por ID y dejan de emitir acciones de
entrada al usar `.disabled()` o `.enabled(false)`; el host sigue manejando
`UiAction::InputChanged`.

`TextArea::preserve_line_endings(true)` conserva CRLF y CR originales en el
valor, clipboard y offsets de edición; se dibujan como saltos de línea visuales
sin reemplazar sus bytes. Su valor predeterminado es `false` y mantiene la
normalización a LF de los formularios existentes. El control nativo de stdin
activa esta opción para confirmar y entregar exactamente la entrada escrita;
la opción no agrega saltos de línea ni ejecuta el proceso.

Las variantes visuales son `Default`, `Secondary`, `Outline`, `Ghost`,
`Destructive`, `Link`, `Warning` y `Success`. `Link` se subraya al recibir hover
o foco de teclado. `ButtonSize` ofrece `Default` (36 px), `Small` (32 px),
`Large` (40 px), `Icon` (36 px), `IconSmall` (32 px) e `IconExtraSmall` (24 px)
mediante `.button_size(...)`. `Subtle`, `Raised` y `Solid`, junto con
`.size(ControlSize)`, siguen disponibles para los controles existentes.

```rust
use lienzo::{Button, ButtonContent, ButtonVariant, Component,
                 ComponentPalette, Icon};

let action = Button::with_content(
    "project:open",
    ButtonContent::IconAndLabel(Icon::Folder, "Abrir proyecto".into()),
    ComponentPalette::neutral_dark(),
)
.variant(ButtonVariant::Raised)
.tooltip("Abrir proyecto")
.tooltip_shortcut("Ctrl+O")
.render();
```

`Tabs`, `Popover` y `DropdownList` implementan `Component::render()` y devuelven
un `Element` ordinario. Reciben `ComponentPalette` del tema y admiten contenido
o acciones de cualquier vista. `Tab` y `DropdownOption` conservan los IDs de
acción del dominio; el host procesa los `UiAction::Clicked` como cualquier otro
botón. El estado `active` y `open` lo conserva el host entre reconstrucciones,
mientras `UiTree` retiene foco, hover y scroll por ID. `ControlSize` define
alturas de 24/28/32/36 px lógicos; `ControlState::Disabled` representa una opción
visual sin región interactiva.

`DropdownMenu` reúne acciones, encabezados y separadores en un popover con un
único ID de cierre. Cada `DropdownMenuItem` puede tener icono, atajo visible,
selección o estado deshabilitado. `DropdownList` adapta sus opciones al mismo
componente para compartir columnas, navegación y movimiento. `MenuMotion`
controla la entrada del popover y la transición de hover/pulsación de cada
fila; `UiTree::set_reduced_motion(true)` los muestra sin animación. El host
mantiene el estado abierto y procesa los IDs de las acciones.
`.header(element)` coloca navegación y búsqueda por encima de las opciones;
el scroll conserva su ID y sólo desplaza la lista. La cabecera queda visible
y mantiene foco y acciones aunque se alcance el final de las opciones.
`.view_key(...)` identifica los cambios de contenido dentro del menú abierto:
reutiliza el fundido de 180 ms y el ajuste de tamaño de 260 ms del popover.
El selector de modelos cambia esa clave al plegar proveedores y excluye el
texto del filtro de la clave; movimiento reducido omite ambas transiciones.
El popover pinta una sola superficie; las filas normales son transparentes y
solo la fila bajo el puntero o seleccionada recibe un fondo. Así el fundido de
entrada y salida no superpone capas de color en todas las filas.
Una fila puede declarar `.submenu(vec![...])`; el panel secundario aparece al
pasar el puntero o enfocar la fila, se coloca a la derecha y cambia a la
izquierda si falta espacio. Flecha derecha entra al submenú y flecha izquierda
regresa a su fila. Encabezados y separadores también funcionan dentro de él.
`DropdownMenuEntry::custom(element)` permite incluir controles propios cuando
hacen falta. La variante `Custom` guarda el elemento en un `Box` para mantener
compactas las entradas ordinarias; el renderer conserva sus IDs y acciones.
Las acciones destructivas pueden
usar `.destructive(color)`.

`ContextMenu::new(id, child, entries, palette)` agrega acciones de clic derecho
sin cambiar la geometría del hijo. `ContextMenuItem` admite icono, atajo y
estado habilitado; `UiTree` conserva la apertura, selección y navegación por
teclado (Shift+F10, flechas, Enter y Escape). Los controles de texto tienen
menú automático de copiar/cortar/pegar/seleccionar, con restricciones para
contraseñas y editores de sólo lectura. El host atiende `CopyRequested` y
`PasteRequested`, y devuelve el texto mediante `UiEvent::PasteTo` sin mover la
selección. Véase el [estándar de menús contextuales](../../docs/ui-selector-standard.md#menús-contextuales-y-edición-de-texto)
para ámbito, movimiento reducido y acciones propias de terminal.

El cursor de campos y editores comparte un reloj de 500 ms por fase. El host
envía el foco nativo con `set_window_focused(...)` y programa el siguiente
repintado usando `animation_repaint_after()` (incluye `caret_repaint_after()`).
Escribir o mover el cursor reinicia la fase visible; reconstruir el árbol
con el mismo contenido la conserva. Selección, menús, recorte y falta de foco
suspenden el temporizador; movimiento reducido mantiene el cursor visible.

`Chip` es una etiqueta pasiva de altura fija (22 px por defecto), esquinas de
radio igual a la mitad de su altura y ancho medido por el host. Un contador de
un dígito ASCII usa un círculo; los demás textos conservan 8 px por lado.
`.height(...)` adapta su altura. Se coloca en una fila y no añade acciones,
foco, hover ni cierre. Marketplace lo usa para contadores y ámbito de instalación.

Los wrappers de `Tooltip` y `Popover` conservan ancho, alto, mínimos y flex de
su ancla. No duplican su padding ni pintura. Así, añadir ayuda o abrir un menú
mantiene las dimensiones de un Switch o Button dentro de una fila alta.

`Alert` es el aviso global reutilizable para información, éxito, advertencia,
error y mensajes neutrales. Recibe título, descripción y `ComponentPalette`;
el mensaje se ajusta al ancho sin altura fija. `.action(element)` admite una
acción existente, `.dismiss(id)` agrega el cierre Ghost estándar y `.compact(true)`
coloca la acción debajo del texto. Cada estado tiene icono, título explícito y
acento de color sobre una superficie neutral. No roba foco, no ejecuta acciones
al pulsar el cuerpo y no desaparece por tiempo: su estado pertenece al host.
Consulta [el diseño y la integración](../../docs/alerts.md).

`AlertDialog` muestra una superficie modal centrada con título, descripción,
cancelación, cierre y acción principal. El fondo atenuado bloquea acciones de
la pantalla; Escape cancela y un clic fuera no cierra el diálogo. El foco queda dentro
del diálogo y vuelve al control anterior al cerrarlo. El host conserva el
estado abierto y ejecuta la operación sólo tras recibir el ID de confirmación.
`.body(element)` agrega un campo u otro contenido entre la descripción y el
pie; el renombrado de tareas y grupos usa esta forma en lugar de un menú lateral.

`Composer` ofrece un editor multilínea controlado y espacios para contexto,
adjuntos y pie. El host suministra el texto, alto, ancho y colores en cada
reconstrucción; `UiTree` conserva el foco y el cursor por ID. Esta separación
permite usar la misma tarjeta en inicio y conversación sin trasladar la lógica
de modelos, permisos o envío al motor de UI.
Con `.pill_controls(...)`, los adjuntos quedan arriba, el editor usa todo el
ancho y las acciones ocupan una fila inferior de 36 px. El texto crece sin
reservar columnas vacías a los lados para los controles.
`.inline_controls(true)` permite una barra inicial compacta con el editor
vacío entre las acciones. El host cambia de disposición al recibir texto y
conserva el mismo ID del editor. `.editor_font_size(...)` ajusta su tipografía;
`.editor_action(...)` reserva un espacio aparte para ampliar o reducir el
editor, sin superponer un botón sobre el texto.
`.auto_grow(min_height, max_height)` usa las métricas de fuente del host y el
ancho disponible para crecer por líneas reales. El elemento `text_area` ofrece
el mismo comportamiento mediante `.text_area_auto_grow(...)`; después del
máximo, conserva su scroll interno para seguir el cursor.

`Attachment` presenta un archivo con icono, título y descripción; la orientación
`Vertical` permite mostrar una imagen `RasterImage` sobre su nombre. Recibe la
paleta del tema y un ID de acción independiente mediante `.remove_action(...)`.
El tooltip conserva el nombre completo y `.help(...)` agrega los detalles.
`.preview_only()` ofrece una miniatura cuadrada con los detalles en el tooltip;
`.open_action(...)` declara la acción al pulsarla. El botón de quitar conserva
un tamaño de 24 × 24 px y su acción tiene prioridad sobre la apertura.
El componente no lee archivos ni el portapapeles: el host entrega los metadatos
y la miniatura. La integración del composer está descrita en
[Adjuntos del composer](../../docs/composer-attachments.md).

```rust
use lienzo::{Attachment, Component, ComponentPalette, Icon};

let file = Attachment::new("attachment:notes", "notas.md", ComponentPalette::neutral_dark())
    .description("MD · 12 KB")
    .icon(Icon::File)
    .remove_action("attachment:notes:remove")
    .width(220.0)
    .render();
```

`ImageViewer` presenta una imagen completa dentro de una superficie modal
ajustada al viewport. El host conserva la selección y entrega la imagen con
`.image(...)`, o un error con `.error(...)`; sin imagen ni error se muestra
el indicador de carga. `.gallery(index, total)`, `.previous_action(...)` y
`.next_action(...)` configuran la navegación. Escape cierra y `.return_focus(...)`
devuelve el foco al control indicado. Las flechas del teclado las procesa el
host para despachar las mismas acciones.
El renderer debe distinguir `RasterImage::is_full_resolution()` de las
miniaturas, verificar los bytes y aplicar un presupuesto acotado de
decodificación y caché para la resolución completa.

`DropdownList` pinta cada opción como una única región de 36 px de alto con
icono de 16 px, etiqueta recortada y marca de selección al extremo derecho.
Reserva las mismas columnas para todas las filas, incluso cuando una opción
carece de icono; cambiar la selección no mueve el texto. El disparador puede
ser cualquier `Element` y `.placement(PopoverPlacement::BottomEnd)` alinea
el menú con su borde derecho. `SegmentedControl` mantiene el radio del track
en 9 px para acompañar los 7 px de los tabs y el inset de 2 px.
En listas que superan ocho opciones, flechas, Home/End y Tab desplazan el
`ScrollArea` para revelar la opción enfocada.

`ScrollArea::uniform_list` construye solo las filas visibles; la lista de
tareas y el árbol de archivos lo usan para mantener estable el trabajo por
cuadro. Por defecto, el scrollbar reserva un carril de 14 px con flechas en
ambos extremos. Un clic desplaza 48 px; mantener pulsado repite el movimiento
tras una breve espera. Las flechas se atenúan en los límites y se omiten en
viewports demasiado cortos. El contenido y sus hovers terminan antes del
carril. `.buttons(false)` permite omitir las flechas, y
`ScrollbarAppearance::overlay()` solicita explícitamente una barra superpuesta.

La rueda, las flechas y los clics de página usan `ScrollMotion` de `motion.rs`:
140 ms con `OutCubic` por defecto. Los gestos sucesivos acumulan el destino y
la transición continúa desde la posición actual. `.motion(ScrollMotion {
duration_ms: 0, ..Default::default() })` desactiva la interpolación en un área.
`UiEvent::ScrollDirect` conserva la respuesta inmediata del touchpad; el host
nativo lo usa para deltas precisos en píxeles. Arrastrar el thumb, revelar el
foco y los métodos `scroll_to_end`/`scroll_to_index` también posicionan de
inmediato. El movimiento reducido ajusta al destino y detiene la animación.

Las áreas anidadas reparten un gesto de rueda: al llegar un panel a su
límite, el desplazamiento restante pasa al contenedor exterior en el mismo
evento, usando el destino pendiente para consumirlo correctamente durante
una animación. Los deltas no finitos se descartan y tanto posición como
destino se limitan al contenido actual. `scroll_offset` devuelve la posición
visible; `is_scroll_at_end` considera el destino para conservar la intención
de lectura al recibir contenido nuevo.
Los menús flotantes mantienen el gesto dentro de su propia superficie: llegar
al final no desplaza la página que queda debajo. Los modales también bloquean
la rueda sobre su fondo.

```rust
use lienzo::{Component, ComponentPalette, DropdownList, DropdownOption,
                  Element, Icon, Popover, PopoverPlacement, Tab, Tabs};

let palette = ComponentPalette::neutral_dark();
let tabs = Tabs::new(vec![
    Tab::new("view:sessions", "Sesiones", 110.0).icon(Icon::Chat).active(true),
    Tab::new("view:files", "Archivos", 110.0).icon(Icon::Folder),
], palette).render();

let picker = DropdownList::new(
    Element::button("mode:toggle", "Modo"),
    vec![DropdownOption::new("mode:plan", "Plan").selected(true)],
    true, "mode:dismiss", palette,
).width(220.0).render();

let menu = Popover::new(tabs, picker, true, "menu:dismiss", palette)
    .placement(PopoverPlacement::BottomEnd)
    .render();
```

`Element::popover` ancla el contenido flotante al rectángulo real del control,
lo coloca sobre el resto de la escena, escapa del recorte de listas con scroll,
se adapta al borde de la ventana y cambia de lado vertical cuando no cabe. Un
clic fuera emite `Clicked` con el ID de cierre; Escape emite la misma acción.
La superficie flotante bloquea el cierre cuando se pulsa en un espacio vacío
dentro del menú. El backend debe pasar Escape como `UiEvent::Key` y repintar
después de actualizar el estado `open`.

Los rótulos de `Button` e `IconButton` se recortan con `truncate_measured`
usando el medidor de texto del renderer. El recorte conserva grafemas Unicode
completos y se dibuja dentro del control sin alterar su región interactiva.

## Editor de código y diff

`CodeEditor` es el editor nativo virtualizado sobre `lienzo-editor`. El host
conserva un `EditorController` con `Arc<Mutex<TextBuffer>>`, construye el mismo
ID en cada frame y administra la carga/guardado del archivo. El componente
obtiene snapshots baratos y sólo pinta filas y columnas visibles; no convierte
el documento completo en un TextArea ni en acciones `InputChanged`.

```rust
use lienzo_editor::TextBuffer;
use lienzo::{CodeEditor, CodeLanguage, Component, EditorController};

let controller = EditorController::new(TextBuffer::new("fn main() {}\n")?);
let editor = CodeEditor::new("file:main.rs", controller.clone())
    .language(CodeLanguage::Rust)
    .height(320.0)
    .render();
```

Por defecto usa fuente mono de 13 px, filas de 20 px, viewport de 320 px y tabs
de cuatro columnas. `.height(...)`, `.tab_width(...)`, `.line_numbers(...)`,
`.read_only(...)` y `.palette(...)` adaptan el componente. También puede usar
`.render().flex(1.0)` dentro de un dock con altura asignada. El backend debe
medir y pintar con la misma fuente mono mediante `set_text_measurer`.

El puntero, Shift+clic/arrastre, flechas, Home/End y PageUp/PageDown conservan
selecciones en índices `char` y límites de grafemas. Tab/Shift+Tab indenta;
Enter inserta el salto de línea del documento. Ctrl+A/C/X/V, Ctrl+Z,
Ctrl+Shift+Z/Ctrl+Y y Ctrl+S procesan selección, clipboard, historial y solicitud
de guardado. Los bytes UTF-8 y saltos LF/CRLF/CR originales se conservan; el
pegado no normaliza el código. Una copia se limita a 1.048.576 caracteres
seleccionados. `read_only` permite leer, navegar y copiar sin modificar el buffer.

El backend entrega `UiEvent::Paste` para distinguir un pegado de la escritura,
`PointerDownModified { shift, .. }` para extender una selección y `Scroll2D`
para deltas horizontal/vertical. Los eventos nuevos mantienen el contrato de
TextArea y scroll fuera del editor. El componente emite `EditorChanged` y
`EditorSaveRequested` con ID de vista, ID de documento y revisión; el host
verifica esa identidad antes de actuar. Los errores llegan por `EditorFailed`.
Los métodos `input_value` y `input_cursor` siguen siendo APIs de formularios;
para código se consulta el buffer/snapshot y su `Selection`.

`UiTree::set_root` conserva el viewport de la misma vista/documento y lo reinicia
al reemplazar el documento. `forget_code_view(id)` libera su estado al cerrar;
`reveal_code_caret(id)` solicita mostrar el cursor después de navegar desde el
host. El caret parpadea con un reloj que se reinicia al escribir o moverlo;
con movimiento reducido permanece visible. El árbol retiene hasta 16 editores y
64 diffs y libera las vistas ocultas más antiguas.

`.language(CodeLanguage::from_extension(extensión))` activa el análisis
Tree-sitter de `lienzo-language`. Incluye Rust, JSON, JavaScript, TypeScript,
TSX, Python, TOML, CSS y HTML; otros archivos usan texto plano. La caché usa un
pool acotado de workers, cancela solicitudes obsoletas y rechaza resultados de
otro documento, revisión o lenguaje. Limita filas y spans; el texto restante
sigue disponible sin resaltado. `.highlights(document_id, revision, ranges)`
y `.diagnostics(document_id, revision, ranges)` permiten que el host comparta
análisis y diagnósticos versionados con el componente.

`EditorController::execute(EditorCommand)` ofrece búsqueda/reemplazo, comandos
de líneas, selecciones múltiples, snippets, comentarios e indentación. Alt+clic
agrega cursores, Alt+Shift crea selección rectangular, doble/triple clic selecciona
palabras/líneas. `.soft_wrap(...)` y `.folds(...)` usan el mismo mapa visual que
el caret y la selección. `.indent_style(...)` y `.auto_pairs(...)` configuran
la escritura. Los eventos `PointerDownEditor`, `ImePreedit`, `ImeCommit`,
`ImeCancelled` e `ImeDeleteSurrounding` conservan el documento propietario de la
composición; el backend obtiene la posición del IME mediante `editor_ime_rects()`.

`.context_actions(...)` añade acciones reales del host al menú contextual de
edición. El host de escritorio integra breadcrumbs, outline, buscar/reemplazar,
preferencias, recuperación de borradores y los servidores LSP de
`lienzo-language`; el componente visual no inicia procesos ni accede a archivos.

`CodeDiff::new(id, Arc<[CodeDiffLine]>)` muestra filas inmutables `Context`,
`Added` y `Removed`, con `old_line`, `new_line` y texto exacto. Comparte tamaños,
paleta, tabs, scroll, clipping y `.language(...)` con el editor. El host conserva
la misma Arc entre frames y decide la procedencia de cada línea. El componente
es de sólo lectura y no ejecuta, aplica ni restaura patches. Las dos versiones
mantienen estado léxico separado; un header de hunk reinicia ambos porque el
contexto omitido es desconocido.

El core limita el documento a 8 MiB por defecto; Agents Window carga archivos
completos de hasta 16 MiB con límites explícitos, trabajo de archivos fuera del
hilo visual, guardado de snapshots y detección de conflictos. Los diffs por chat
se abren a pedido y usan efectos confirmados del journal, con cobertura parcial
visible. Consulta [el diseño y validación](../../docs/native-code-editor-design.md)
y [guardado e historial](../../docs/editor-save-history.md) para límites,
autoridad de archivos y cómo abrir, guardar o recargar un documento.

## Contexto

`ContextMeter::new(id, percent, palette)` combina un anillo de uso y un control
accesible por pointer/teclado. `None` conserva el estado desconocido; `Some(0)` y
`Some(100)` dibujan uso vacío/completo. El host entrega el porcentaje de uso;
el componente no consulta proveedores ni calcula tokens.

`ContextContent` compone secciones con `ContextRow` (etiqueta, valor y descripción)
y footer. Conserva unidades, precisión y procedencia del host. Se puede colocar
dentro de ScrollArea/Popover; texto y hashes largos se ajustan al ancho. No hay
inferencia de capacidades por nombre, tokenizer ni base de precios en la UI.
Las pruebas verifican unknown/zero/full, activación por teclado y contenido largo
a 240/360/640 px. El host verifica además scopes, scroll y Escape del panel.

`ContextUsage` es el resumen compacto: porcentaje/cociente aportados por el host,
barra de `ContextSegment` con fracciones medidas, leyenda de `ContextUsageRow` y
acciones explícitas de cerrar/detalles. Las fracciones se limitan a la ventana;
bytes, cantidades o valores desconocidos no crean segmentos de tokens. La prueba
de geometría verifica proporciones 25%/5%/70% a 300/660 px y acciones por teclado.
El host usa este resumen como vista inicial y `ContextContent` como inspección
expandida, con offsets de scroll separados.

### Menús dentro de un panel

`DropdownMenu::inline(true)` conserva el disparador y coloca las opciones
expandidas debajo, ocupando espacio en el layout. No crea backdrop ni un
popover sobre otras columnas. La lista conserva su altura acotada y scroll
cuando supera `max_visible_items`; el host conserva el estado abierto y la
acción de cierre. Entorno utiliza esta presentación para sus selectores.

## Arrastrar y soltar

`Element::draggable()` marca un botón o `clickable` como origen y
`Element::drop_target()` marca un destino (botón, scroll o lista virtual). Un
clic con menos de 4 px de recorrido sigue siendo `Clicked`. Al superarlo,
`UiTree` emite `DragStarted`, luego `DragMoved { over }` en cada movimiento y
`Dropped { target }` al soltar; Escape o `PointerLeave` emiten `DragCancelled`.
`over` y `target` son el destino más alto bajo el puntero, nunca el origen.
Mientras dura el arrastre, el motor repinta el origen encima de todo, desplazado
con el puntero y al 80 % de opacidad; no añade regiones interactivas. La imagen
se captura una vez al empezar, así que sigue visible aunque la fila de origen
salga de la vista o una lista virtual la recicle.

Para archivos que llegan desde otra aplicación, el host envía
`UiEvent::FilesHovered` y `UiEvent::FilesDropped`, y recibe las acciones del
mismo nombre con el destino resuelto.

En Wayland, winit no habla el protocolo de arrastre, así que la ventana no
recibe archivos ni puede iniciar un arrastre. `lienzo-dnd` lo resuelve con una
cola de eventos propia sobre el `wl_display` de la ventana:
`FileDnd::attach(ventana, despertar)` devuelve `None` fuera de Wayland;
`poll()` entrega `Hovered { x, y }` en cada movimiento, `Left` y
`Dropped { x, y, paths }`, que el host reenvía como `FilesHovered` y
`FilesDropped`. `start_drag(paths)` entrega los archivos al compositor para
soltarlos en otra aplicación; hay que llamarlo con el botón aún presionado, por
ejemplo al recibir `DragStarted`. Desde ahí el compositor es dueño del puntero:
el arrastre interno se cancela y un drop sobre la propia ventana vuelve como
`Dropped`. El arrastre saliente en X11, Windows y macOS no está implementado.

## Resortes

`ScalarSpring` anima un valor con un resorte de forma cerrada. A diferencia de
`ScalarTransition`, `retarget` conserva posición y velocidad, y
`release(valor, velocidad, destino, ahora)` continúa un arrastre soltado.
`SpringMotion { damping_ratio, stiffness }` ajusta la física; 1.0 no rebota.

## Features

`syntax` (activo por defecto) incluye `lienzo-language` y tree-sitter. Sin él,
las vistas de código usan el lexer por línea y `CodeLanguage` es local.

## Tema

`Theme { background, foreground, accent }` deriva toda la paleta de tres
colores, siguiendo el modelo de Warp: `surface(1..=7)` mezcla el fondo con 5,
10, 15, 20, 40, 60 y 90 % del primer plano; `text`, `text_sub`, `text_hint` y
`text_disabled` son el primer plano al 90, 60, 40 y 20 %; `hover` y `pressed`
son velos translúcidos; `accent_surface` y `accent_surface_strong` diluyen el
acento en el fondo al 40 y 60 %. Todo es `const fn`, y `palette()` entrega el
`ComponentPalette` de los componentes. `Theme::DARK` es un gris neutro.
