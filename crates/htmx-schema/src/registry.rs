use crate::attribute::{
    AttributeCategory, AttributeValueType, HtmxAttributeDefinition, ModifierDefinition,
};

pub static SWAP_STYLES: &[&str] = &[
    "innerHTML",
    "outerHTML",
    "textContent",
    "beforebegin",
    "afterbegin",
    "beforeend",
    "afterend",
    "delete",
    "none",
];

pub static TARGET_KEYWORDS: &[&str] = &["this", "closest", "find", "next", "previous"];

pub static TRIGGER_EVENTS: &[&str] = &[
    "click",
    "dblclick",
    "change",
    "submit",
    "keyup",
    "keydown",
    "mouseenter",
    "mouseleave",
    "load",
    "revealed",
    "intersect",
];

pub static SWAP_MODIFIERS: &[ModifierDefinition] = &[
    ModifierDefinition {
        name: "transition",
        description: "Enables View Transitions API for the swap.",
        takes_value: true,
        allowed_values: &["true", "false"],
    },
    ModifierDefinition {
        name: "swap",
        description: "Time to wait before swapping content (e.g. `swap:500ms`).",
        takes_value: true,
        allowed_values: &["100ms", "200ms", "500ms", "1s"],
    },
    ModifierDefinition {
        name: "settle",
        description: "Time to wait after swap before settling (e.g. `settle:100ms`).",
        takes_value: true,
        allowed_values: &["100ms", "200ms", "500ms", "1s"],
    },
    ModifierDefinition {
        name: "scroll",
        description: "Scroll the target element or window to top or bottom.",
        takes_value: true,
        allowed_values: &["top", "bottom"],
    },
    ModifierDefinition {
        name: "show",
        description: "Scroll the viewport to make the target element visible.",
        takes_value: true,
        allowed_values: &["top", "bottom"],
    },
    ModifierDefinition {
        name: "focus-scroll",
        description: "Whether to scroll smoothly to newly focused inputs.",
        takes_value: true,
        allowed_values: &["true", "false"],
    },
    ModifierDefinition {
        name: "ignoreTitle",
        description: "Disables automatic title update when response contains a <title> tag.",
        takes_value: true,
        allowed_values: &["true", "false"],
    },
];

pub static TRIGGER_MODIFIERS: &[ModifierDefinition] = &[
    ModifierDefinition {
        name: "once",
        description: "The request will only be triggered once.",
        takes_value: false,
        allowed_values: &[],
    },
    ModifierDefinition {
        name: "changed",
        description: "Only trigger if the value of the element has changed.",
        takes_value: false,
        allowed_values: &[],
    },
    ModifierDefinition {
        name: "delay",
        description: "Wait the specified duration before triggering (debounces input).",
        takes_value: true,
        allowed_values: &["100ms", "200ms", "500ms", "1s"],
    },
    ModifierDefinition {
        name: "throttle",
        description: "Wait specified duration before allowing another request.",
        takes_value: true,
        allowed_values: &["100ms", "200ms", "500ms", "1s"],
    },
    ModifierDefinition {
        name: "from",
        description: "Listen for event on a different element (e.g. `from:body`, `from:document`).",
        takes_value: true,
        allowed_values: &["document", "window", "body", "closest"],
    },
    ModifierDefinition {
        name: "target",
        description: "Filter event by target CSS selector.",
        takes_value: true,
        allowed_values: &[],
    },
    ModifierDefinition {
        name: "consume",
        description: "Stop event propagation (calls `event.stopPropagation()`).",
        takes_value: false,
        allowed_values: &[],
    },
    ModifierDefinition {
        name: "queue",
        description: "Determines how events are queued when a request is already in flight.",
        takes_value: true,
        allowed_values: &["first", "last", "all", "none"],
    },
    ModifierDefinition {
        name: "root",
        description:
            "CSS selector of the root element for intersection observer (`root:<selector>`).",
        takes_value: true,
        allowed_values: &[],
    },
    ModifierDefinition {
        name: "threshold",
        description: "Intersection ratio threshold between 0.0 and 1.0 (`threshold:<number>`).",
        takes_value: true,
        allowed_values: &["0.0", "0.25", "0.5", "0.75", "1.0"],
    },
];

pub static ATTRIBUTES: &[HtmxAttributeDefinition] = &[
    HtmxAttributeDefinition {
        name: "hx-get",
        category: AttributeCategory::Core,
        value_type: AttributeValueType::Url,
        summary: "Issues a GET request to the specified URL",
        description: "The `hx-get` attribute issues a `GET` request to the given URL when triggered by the element.

### Example
```html
<button hx-get=\"/api/contacts\" hx-target=\"#contact-list\">
    Load Contacts
</button>
```",
        allowed_values: &[],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-get/",
    },
    HtmxAttributeDefinition {
        name: "hx-post",
        category: AttributeCategory::Core,
        value_type: AttributeValueType::Url,
        summary: "Issues a POST request to the specified URL",
        description: "The `hx-post` attribute issues a `POST` request to the given URL when triggered. Useful for submitting forms or initiating state-changing actions.

### Example
```html
<form hx-post=\"/submit\" hx-target=\"#result\">
    <input name=\"email\" type=\"email\" />
    <button type=\"submit\">Submit</button>
</form>
```",
        allowed_values: &[],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-post/",
    },
    HtmxAttributeDefinition {
        name: "hx-put",
        category: AttributeCategory::Core,
        value_type: AttributeValueType::Url,
        summary: "Issues a PUT request to the specified URL",
        description: "The `hx-put` attribute issues an HTTP `PUT` request to update a server resource.",
        allowed_values: &[],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-put/",
    },
    HtmxAttributeDefinition {
        name: "hx-delete",
        category: AttributeCategory::Core,
        value_type: AttributeValueType::Url,
        summary: "Issues a DELETE request to the specified URL",
        description: "The `hx-delete` attribute issues an HTTP `DELETE` request to delete a server resource.",
        allowed_values: &[],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-delete/",
    },
    HtmxAttributeDefinition {
        name: "hx-patch",
        category: AttributeCategory::Core,
        value_type: AttributeValueType::Url,
        summary: "Issues a PATCH request to the specified URL",
        description: "The `hx-patch` attribute issues an HTTP `PATCH` request for partial resource modifications.",
        allowed_values: &[],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-patch/",
    },
    HtmxAttributeDefinition {
        name: "hx-target",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::CssSelector,
        summary: "Specifies the target element to swap content into",
        description: "The `hx-target` attribute allows you to target an element for swapping other than the element that triggered the request.

Supported target syntax:
- `#id`: Target element by CSS ID selector.
- `.class`: Target element by CSS class selector.
- `this`: The element itself.
- `closest <selector>`: The closest ancestor matching selector.
- `find <selector>`: The first descendant matching selector.
- `next <selector>`: Next sibling matching selector.
- `previous <selector>`: Previous sibling matching selector.

### Example
```html
<button hx-get=\"/info\" hx-target=\"#details\">Show Info</button>
<div id=\"details\"></div>
```",
        allowed_values: TARGET_KEYWORDS,
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-target/",
    },
    HtmxAttributeDefinition {
        name: "hx-swap",
        category: AttributeCategory::Core,
        value_type: AttributeValueType::SwapStyle,
        summary: "Controls how the returned content is swapped into the DOM",
        description: "The `hx-swap` attribute allows you to specify how the response will be swapped into the target element.

### Available Styles
- `innerHTML`: Replace inner HTML of target (default).
- `outerHTML`: Replace entire target element with response.
- `beforebegin`: Insert response before the target element.
- `afterbegin`: Insert response before the first child of target.
- `beforeend`: Insert response after the last child of target.
- `afterend`: Insert response after the target element.
- `delete`: Deletes the target element regardless of response.
- `none`: Does not append response (still processes out-of-band swaps).

### Modifiers
- `transition:true`: Enable View Transitions.
- `swap:500ms`: Delay swap timing.
- `settle:200ms`: Delay settle timing.
- `scroll:top|bottom`: Scroll after swap.
- `show:top|bottom`: Make target visible.

### Example
```html
<button hx-get=\"/items\" hx-target=\"#list\" hx-swap=\"beforeend transition:true\">
    Add Item
</button>
```",
        allowed_values: SWAP_STYLES,
        modifiers: SWAP_MODIFIERS,
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-swap/",
    },
    HtmxAttributeDefinition {
        name: "hx-trigger",
        category: AttributeCategory::Core,
        value_type: AttributeValueType::TriggerSpecification,
        summary: "Specifies the event that triggers the request",
        description: "The `hx-trigger` attribute allows you to specify what event triggers an HTTP request.

### Syntax
`hx-trigger=\"<event-name> [filter] [modifier]*\"`

Common events: `click`, `change`, `submit`, `keyup`, `load`, `revealed`, `intersect`.

Modifiers: `once`, `changed`, `delay:<time>`, `throttle:<time>`, `from:<selector>`, `target:<selector>`, `consume`, `queue:<type>`.

### Example
```html
<input type=\"text\" name=\"q\"
       hx-get=\"/search\"
       hx-trigger=\"keyup changed delay:500ms\"
       hx-target=\"#search-results\" />
```",
        allowed_values: TRIGGER_EVENTS,
        modifiers: TRIGGER_MODIFIERS,
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-trigger/",
    },
    HtmxAttributeDefinition {
        name: "hx-indicator",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::CssSelector,
        summary: "Element to put the htmx-request class on during a request",
        description: "The `hx-indicator` attribute allows you to specify an element that will have the `htmx-request` class added to it while the request is in flight.

### Example
```html
<button hx-get=\"/data\" hx-indicator=\"#spinner\">Load</button>
<img id=\"spinner\" class=\"htmx-indicator\" src=\"/spinner.gif\" />
```",
        allowed_values: &["closest", "this"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-indicator/",
    },
    HtmxAttributeDefinition {
        name: "hx-boost",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::Boolean,
        summary: "Progressively enhance links and forms into AJAX requests",
        description: "The `hx-boost` attribute converts standard anchor links and form submissions into AJAX requests transparently.

### Example
```html
<div hx-boost=\"true\">
    <a href=\"/blog\">Blog</a>
    <a href=\"/about\">About</a>
</div>
```",
        allowed_values: &["true", "false"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-boost/",
    },
    HtmxAttributeDefinition {
        name: "hx-push-url",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::Boolean,
        summary: "Pushes the request URL into the browser navigation history",
        description: "The `hx-push-url` attribute pushes the request URL into the browser navigation history, creating a new history entry.",
        allowed_values: &["true", "false"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-push-url/",
    },
    HtmxAttributeDefinition {
        name: "hx-select",
        category: AttributeCategory::Core,
        value_type: AttributeValueType::CssSelector,
        summary: "Selects a subset of the response HTML to swap",
        description: "The `hx-select` attribute allows you to select a subset of the server response HTML to swap into the target, discarding the rest.",
        allowed_values: &[],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-select/",
    },
    HtmxAttributeDefinition {
        name: "hx-select-oob",
        category: AttributeCategory::Core,
        value_type: AttributeValueType::CssSelector,
        summary: "Selects content from response for out-of-band swap",
        description: "The `hx-select-oob` attribute selects content from a response to be swapped out-of-band directly into the DOM.",
        allowed_values: &[],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-select-oob/",
    },
    HtmxAttributeDefinition {
        name: "hx-swap-oob",
        category: AttributeCategory::Core,
        value_type: AttributeValueType::SwapStyle,
        summary: "Marks an element in response for out-of-band swap",
        description: "The `hx-swap-oob` attribute allows you to specify that an element in a response should be swapped into the DOM out-of-band.",
        allowed_values: &["true", "innerHTML", "outerHTML", "beforebegin", "afterbegin", "beforeend", "afterend", "delete"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-swap-oob/",
    },
    HtmxAttributeDefinition {
        name: "hx-vals",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::JsonOrExpression,
        summary: "Adds parameters to submit with the request",
        description: "The `hx-vals` attribute allows you to add parameters to an AJAX request, either via JSON or JavaScript evaluation (with `js:` prefix).

### Example
```html
<div hx-get=\"/api\" hx-vals='{\"myVal\": \"foo\"}'>Submit</div>
```",
        allowed_values: &[],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-vals/",
    },
    HtmxAttributeDefinition {
        name: "hx-headers",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::JsonOrExpression,
        summary: "Adds custom HTTP headers to the request",
        description: "The `hx-headers` attribute allows you to add custom HTTP headers to an AJAX request.",
        allowed_values: &[],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-headers/",
    },
    HtmxAttributeDefinition {
        name: "hx-params",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::Params,
        summary: "Filters the parameters submitted with an AJAX request",
        description: "The `hx-params` attribute allows you to filter the parameters submitted with an AJAX request (`*`, `none`, `not <param-list>`, `<param-list>`).",
        allowed_values: &["*", "none", "not "],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-params/",
    },
    HtmxAttributeDefinition {
        name: "hx-include",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::CssSelector,
        summary: "Includes additional element values in the request",
        description: "The `hx-include` attribute allows you to include additional element values with an AJAX request.",
        allowed_values: &["this", "closest", "find"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-include/",
    },
    HtmxAttributeDefinition {
        name: "hx-sync",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::SyncSpecification,
        summary: "Coordinates requests between multiple elements",
        description: "The `hx-sync` attribute allows you to coordinate requests across multiple elements to avoid race conditions.

Syntax: `<selector>:<strategy>` where strategy is `drop`, `abort`, `replace`, or `queue`.",
        allowed_values: &["drop", "abort", "replace", "queue first", "queue last", "queue all"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-sync/",
    },
    HtmxAttributeDefinition {
        name: "hx-ext",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::ExtensionList,
        summary: "Enables an HTMX extension for an element and its children",
        description: "The `hx-ext` attribute enables one or more HTMX extensions for an element and its descendants.

### Example
```html
<body hx-ext=\"ws, preload\">
    ...
</body>
```",
        allowed_values: &["ws", "sse", "preload", "response-targets", "head-support", "idiomorph"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-ext/",
    },
    HtmxAttributeDefinition {
        name: "hx-confirm",
        category: AttributeCategory::Additional,
        value_type: AttributeValueType::Text,
        summary: "Shows a confirm() dialog before issuing a request",
        description: "The `hx-confirm` attribute prompts the user with a JavaScript `confirm()` dialog before issuing the request.",
        allowed_values: &[],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-confirm/",
    },
    HtmxAttributeDefinition {
        name: "hx-prompt",
        category: AttributeCategory::Additional,
        value_type: AttributeValueType::Text,
        summary: "Shows a prompt() dialog before issuing a request",
        description: "The `hx-prompt` attribute prompts the user with a JavaScript `prompt()` dialog and submits the entered response in `HX-Prompt` header.",
        allowed_values: &[],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-prompt/",
    },
    HtmxAttributeDefinition {
        name: "hx-validate",
        category: AttributeCategory::Additional,
        value_type: AttributeValueType::Boolean,
        summary: "Forces element validation before issuing request",
        description: "The `hx-validate` attribute forces element validation before issuing request, even on elements that do not normally trigger validation.",
        allowed_values: &["true", "false"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-validate/",
    },
    HtmxAttributeDefinition {
        name: "hx-disinherit",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::Text,
        summary: "Disables inheritance of specified HTMX attributes",
        description: "The `hx-disinherit` attribute allows you to disable attribute inheritance on a subtree (`hx-disinherit=\"*\"` or specific attributes like `hx-disinherit=\"hx-target\"`).",
        allowed_values: &["*", "hx-target", "hx-indicator", "hx-boost"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-disinherit/",
    },
    HtmxAttributeDefinition {
        name: "hx-disabled-elt",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::CssSelector,
        summary: "Disables elements while a request is in flight",
        description: "The `hx-disabled-elt` attribute specifies elements that should be disabled while the request is in flight.",
        allowed_values: &["this", "closest"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-disabled-elt/",
    },
    HtmxAttributeDefinition {
        name: "hx-on",
        category: AttributeCategory::Additional,
        value_type: AttributeValueType::Text,
        summary: "Handles DOM events inline with JavaScript",
        description: "The `hx-on:<event>` attribute allows attaching inline event handlers directly on elements.",
        allowed_values: &[],
        modifiers: &[],
        deprecated: Some("Bare hx-on is deprecated in HTMX 2.0; use hx-on:<event> (e.g. hx-on:click) instead."),
        documentation_url: "https://htmx.org/attributes/hx-on/",
    },
    HtmxAttributeDefinition {
        name: "hx-replace-url",
        category: AttributeCategory::Inherited,
        value_type: AttributeValueType::Boolean,
        summary: "Replaces current URL in browser location history",
        description: "The `hx-replace-url` attribute replaces the current URL in the browser location history.",
        allowed_values: &["true", "false"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-replace-url/",
    },
    HtmxAttributeDefinition {
        name: "hx-encoding",
        category: AttributeCategory::Additional,
        value_type: AttributeValueType::Text,
        summary: "Sets the encoding type for form submission",
        description: "The `hx-encoding` attribute sets multipart encoding (`multipart/form-data`) for file uploads.",
        allowed_values: &["multipart/form-data"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-encoding/",
    },
    HtmxAttributeDefinition {
        name: "hx-preserve",
        category: AttributeCategory::Additional,
        value_type: AttributeValueType::Boolean,
        summary: "Prevents element from being swapped out during updates",
        description: "The `hx-preserve` attribute keeps an element unchanged when a parent element is swapped.",
        allowed_values: &["true"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-preserve/",
    },
    HtmxAttributeDefinition {
        name: "hx-history",
        category: AttributeCategory::Additional,
        value_type: AttributeValueType::Boolean,
        summary: "Prevents sensitive data from entering the history cache",
        description: "The `hx-history=\"false\"` attribute excludes the element or page from the local history cache.",
        allowed_values: &["false"],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-history/",
    },
    HtmxAttributeDefinition {
        name: "hx-history-elt",
        category: AttributeCategory::Additional,
        value_type: AttributeValueType::Boolean,
        summary: "Designates the element to snapshot for history navigation",
        description: "The `hx-history-elt` attribute designates which element to snapshot for history restoration.",
        allowed_values: &[],
        modifiers: &[],
        deprecated: None,
        documentation_url: "https://htmx.org/attributes/hx-history-elt/",
    },
    // Deprecated HTMX 1.x attributes
    HtmxAttributeDefinition {
        name: "hx-ws",
        category: AttributeCategory::Additional,
        value_type: AttributeValueType::Text,
        summary: "Deprecated: WebSocket support in HTMX 1.x",
        description: "⚠️ **Deprecated in HTMX 2.0**: Use `hx-ext=\"ws\"` with `ws-connect` and `ws-send` instead.",
        allowed_values: &[],
        modifiers: &[],
        deprecated: Some("hx-ws is deprecated in HTMX 2.0. Migrate to hx-ext=\"ws\" with ws-connect and ws-send."),
        documentation_url: "https://htmx.org/extensions/web-sockets/",
    },
    HtmxAttributeDefinition {
        name: "hx-sse",
        category: AttributeCategory::Additional,
        value_type: AttributeValueType::Text,
        summary: "Deprecated: Server-Sent Events support in HTMX 1.x",
        description: "⚠️ **Deprecated in HTMX 2.0**: Use `hx-ext=\"sse\"` with `sse-connect` and `sse-swap` instead.",
        allowed_values: &[],
        modifiers: &[],
        deprecated: Some("hx-sse is deprecated in HTMX 2.0. Migrate to hx-ext=\"sse\" with sse-connect and sse-swap."),
        documentation_url: "https://htmx.org/extensions/server-sent-events/",
    },
];

/// Look up an attribute definition by name, supporting both `hx-*` and `data-hx-*`.
pub fn attribute(name: &str) -> Option<&'static HtmxAttributeDefinition> {
    let normalized = HtmxAttributeDefinition::normalize_name(name);
    // Also support hx-on:* prefix pattern
    if normalized.starts_with("hx-on:") {
        return ATTRIBUTES.iter().find(|attr| attr.name == "hx-on");
    }
    ATTRIBUTES.iter().find(|attr| attr.name == normalized)
}

/// Alias for [`attribute`] adhering to legacy `get_` convention.
pub fn get_attribute(name: &str) -> Option<&'static HtmxAttributeDefinition> {
    attribute(name)
}

/// Retrieve all defined attributes.
pub fn all_attributes() -> &'static [HtmxAttributeDefinition] {
    ATTRIBUTES
}
