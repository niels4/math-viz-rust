//! The page URL as app state: the query string plus one [`QueryParam`] per
//! page parameter.
//!
//! Ported from math-viz2 at `c409aad`:
//!
//! - `src/components/router/router-util.ts`: [`trim_slashes`],
//!   [`get_route_and_search`], [`create_full_hash`].
//! - `src/components/router/queryParamAtom.ts`: [`QueryParamDef`] (the
//!   static description), [`QueryParam`] (the live value, the debounce,
//!   flush, equality), the default-flushes-as-deletion rule.
//! - `src/components/router/router-hooks.ts`: the hash read and write only
//!   (the router itself — routes, listeners, `Router.tsx` — is not ported):
//!   the merge with delete-on-empty, the literal-comma rule, `replaceState`
//!   (no history entry), the `hashchange` flow back in.
//!
//! A param's live value updates at once while the URL follows after its
//! debounce, exactly as the hook does. The debounce runs on the app's frame
//! clock, never on a thread: [`QueryParam::set`] arms a deadline in whole
//! milliseconds and [`QueryParam::step`] fires it, so the harness's paused
//! clock drives it like every other motion. Callers pass
//! `(now / app_loop::MS) as u64`; the truncation is consistent between `set`
//! and `step`, so a deadline armed for `debounce_ms` fires exactly there.
//!
//! Wiring contract for the milestones that own params (h2-p-camera,
//! h2-input): each frame, `step` every param, then `sync` every param (an
//! external hash edit flows into the live values), and when `Url::query`
//! changed, write `Url::full_hash` through
//! `platform_web::platform_write_hash` (web only). The `hashchange` export
//! ([`mv_hash_changed`], queued as `UserCode::Hash`) carries route and query
//! in at once; the per-param `sync` rides the frame step.

/// The debounce every param takes unless its def says otherwise: 200 ms, as
/// `queryParamAtom`'s `debounceMs = 200` default (the camera defs at this
/// hash pass 200 explicitly).
pub const DEFAULT_DEBOUNCE_MS: u64 = 200;

/// The static description of one page parameter: `queryParamAtom`'s `key`
/// plus its options. A fn pair carries parse and serialise (plain data, both
/// `Copy`; a trait would work too but every def would then need a type).
/// `serialize` returning `None` deletes the key, as a TS `null` does.
pub struct QueryParamDef<T> {
    /// The key in the query (`center`, `scale`, `func`, …).
    pub key: &'static str,
    /// The value a missing or unparseable key reads as.
    pub default: T,
    /// The URL write waits this long after the last `set`.
    pub debounce_ms: u64,
    /// `(raw: string | null) => T | undefined`: `None` is the default.
    /// `raw` is the decoded value (`URLSearchParams.get`), `None` a missing
    /// key. math-viz2 parses with `Number(raw)`, whose answers for the odd
    /// inputs are: empty/blank → 0, `0x` hex → its value, `Infinity`/`NaN` →
    /// unparseable. [`parse_js_number`] keeps those three.
    pub parse: fn(Option<&str>) -> Option<T>,
    /// `(value: T) => string | number | null`: `None` deletes the key.
    pub serialize: fn(&T) -> Option<String>,
    /// Value equality; a `set` to an equal value is a no-op (it neither
    /// touches the live value nor re-arms the timer).
    pub equals: fn(&T, &T) -> bool,
}

/// One pending debounced URL write: the value serialised at `set` time (as
/// the `setTimeout` closure captures `next`) and its deadline in whole
/// milliseconds on the app's frame clock.
struct Pending {
    serial: Option<String>,
    due_ms: u64,
}

/// One page parameter: `useQueryParam`'s `[value, set, flush]` as state.
/// The live value, the pending write and the `lastRaw` skip all hang here;
/// the query string itself hangs on [`Url`].
pub struct QueryParam<T: 'static> {
    def: &'static QueryParamDef<T>,
    live: T,
    pending: Option<Pending>,
    /// The key's text as last seen in the query (`None` until the first
    /// `sync`/`new`): `sync` skips unless it changed, so a write by another
    /// param (which leaves this key's text alone) never clobbers the live
    /// value — the pan-before-zoom ordering the vitest test pins.
    last_raw: Option<Option<String>>,
}

impl<T: 'static> QueryParam<T> {
    /// Binds the def to the query: the initial value parses as
    /// `queryParamAtom`'s does (`parse(raw) ?? default`), with `last_raw`
    /// seeded, as the hook's mount-time `syncFromUrl` leaves it.
    pub fn new(def: &'static QueryParamDef<T>, query: &str) -> Self
    where
        T: Clone,
    {
        let raw = raw_value(query, def.key);
        let live = (def.parse)(raw.as_deref()).unwrap_or_else(|| def.default.clone());
        Self {
            def,
            live,
            pending: None,
            last_raw: Some(raw),
        }
    }

    /// The live value, at once (the hook's `value`).
    pub fn get(&self) -> &T {
        &self.live
    }

    /// Sets the live value at once and arms the debounced URL write (the
    /// hook's `set`). An equal value is a no-op: the pending write keeps its
    /// earlier deadline and serial, as the hook's early return keeps its
    /// timer. Callers with an updater closure read `get`, compute, `set`.
    pub fn set(&mut self, next: T, now_ms: u64)
    where
        T: Clone,
    {
        if (self.def.equals)(&self.live, &next) {
            return;
        }
        self.live = next;
        let serial = url_value(self.def, &self.live);
        self.pending = Some(Pending {
            serial,
            due_ms: now_ms.saturating_add(self.def.debounce_ms),
        });
    }

    /// Sets the live value and writes the URL now (the hook's `set` with
    /// `{ flush: true }`).
    pub fn set_flushed(&mut self, url: &mut Url, next: T)
    where
        T: Clone,
    {
        if (self.def.equals)(&self.live, &next) {
            return;
        }
        self.live = next;
        self.pending = None;
        let serial = url_value(self.def, &self.live);
        url.write(self.def.key, serial);
        self.last_raw = Some(url.raw(self.def.key));
    }

    /// Writes the current live value now, dropping any pending write (the
    /// hook's gesture-end `flushCurrent`).
    pub fn flush(&mut self, url: &mut Url)
    where
        T: Clone,
    {
        self.pending = None;
        let serial = url_value(self.def, &self.live);
        url.write(self.def.key, serial);
        self.last_raw = Some(url.raw(self.def.key));
    }

    /// Copies an external query change into the live value (the hook's
    /// `syncFromUrl` after a `hashchange`): skipped unless this key's text
    /// changed, and then only when the parsed value serialises differently
    /// (sub-serialise live ticks survive a round trip through the URL).
    /// Returns whether the live value changed.
    pub fn sync(&mut self, url: &Url) -> bool
    where
        T: Clone,
    {
        let raw = url.raw(self.def.key);
        if self.last_raw.as_ref() == Some(&raw) {
            return false;
        }
        let next = (self.def.parse)(raw.as_deref()).unwrap_or_else(|| self.def.default.clone());
        self.last_raw = Some(raw);
        if (self.def.serialize)(&self.live) == (self.def.serialize)(&next) {
            return false;
        }
        self.live = next;
        true
    }

    /// Fires the pending write when due, merging it into the query (one
    /// param's `setTimeout` going off). Returns whether the query changed.
    pub fn step(&mut self, url: &mut Url, now_ms: u64) -> bool {
        let Some(pending) = self.pending.take_if(|p| p.due_ms <= now_ms) else {
            return false;
        };
        url.write(self.def.key, pending.serial);
        self.last_raw = Some(url.raw(self.def.key));
        true
    }
}

/// The `toUrlValue` rule: a value serialising as the default flushes as a
/// deletion, so defaults stay out of the URL and links stay short.
fn url_value<T>(def: &QueryParamDef<T>, live: &T) -> Option<String> {
    let serial = (def.serialize)(live);
    if serial.as_deref() == (def.serialize)(&def.default).as_deref() {
        None
    } else {
        serial
    }
}

/// The query string plus its hash route: `getCurrentSearchParams` and
/// `currentRoute` as data. `App.url` holds one of these.
pub struct Url {
    query: String,
    route: String,
}

impl Url {
    /// Parses a full page hash (`#function-viewer?center=1,2&scale=2`): the
    /// web start reads `location.hash` through `mv_read_hash`.
    pub fn new(hash: &str) -> Self {
        let (route, query) = get_route_and_search(hash);
        Self { query, route }
    }

    /// The bare query (`--query` natively, which carries no hash): no route.
    pub fn from_query(query: &str) -> Self {
        Self {
            query: query.to_string(),
            route: String::new(),
        }
    }

    /// The serialised query (`center=1,2&scale=2`): what `views.json`
    /// records and what the params parse.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// The hash route the query was read with (`function-viewer`): carried
    /// so the web mirror writes the full hash back.
    pub fn route(&self) -> &str {
        &self.route
    }

    /// The full page hash to write (`createFullHash`): the web mirror hands
    /// this to `mv_write_hash`.
    pub fn full_hash(&self) -> String {
        create_full_hash(&self.route, &self.query)
    }

    /// The key's first decoded value (`URLSearchParams.get`), `None` when
    /// missing.
    pub fn raw(&self, key: &str) -> Option<String> {
        raw_value(&self.query, key)
    }

    /// Merges one key into the query (`useSetSearchParams` minus the
    /// location write and the listener fan-out): `Some` sets (an existing
    /// key keeps its position, a new one appends, as `URLSearchParams.set`
    /// does), `None` deletes. The other keys' text is untouched.
    pub fn write(&mut self, key: &str, value: Option<String>) {
        self.query = write_pair(&self.query, key, value);
    }

    /// Reads a fresh page hash in (the `hashchange` path): route and query
    /// both update. Param live values follow through each param's `sync`,
    /// which their owners run on the frame step.
    pub fn sync_from_hash(&mut self, hash: &str) {
        let (route, query) = get_route_and_search(hash);
        self.route = route;
        self.query = query;
    }

    /// The param's live value (the hook's `value`).
    pub fn get<P: Clone>(&self, param: &QueryParam<P>) -> P {
        param.get().clone()
    }

    /// Sets the live value at once and arms the debounced URL write (the
    /// hook's `set`).
    pub fn set<P: Clone>(&mut self, param: &mut QueryParam<P>, next: P, now_ms: u64) {
        param.set(next, now_ms);
    }

    /// Writes the param's current live value now (the hook's gesture-end
    /// `flushCurrent`).
    pub fn flush<P: Clone>(&mut self, param: &mut QueryParam<P>) {
        param.flush(self);
    }
}

/// `defaultEquals`: `Object.is` for scalars (element-wise for arrays).
/// `Object.is` treats `NaN` as equal to itself and `-0` as different from
/// `0`, where `PartialEq` says the opposite on both counts; params are finite
/// by parse, so only the `-0` half can bite (a `set(-0.0)` over a live `0.0`
/// re-arms the debounce, as the TypeScript does — the serialised URL is the
/// same either way).
pub fn default_equals<T: ObjectIs>(a: &T, b: &T) -> bool {
    a.object_is(b)
}

/// `Object.is` semantics for a param value: what [`default_equals`] compares
/// through. Implemented for the value shapes the page params take; a later
/// milestone adds its own shape here (never a second equality fn).
pub trait ObjectIs {
    /// Whether two values are the same value (`Object.is`).
    fn object_is(&self, other: &Self) -> bool;
}

impl ObjectIs for f64 {
    fn object_is(&self, other: &Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl ObjectIs for [f64; 2] {
    fn object_is(&self, other: &Self) -> bool {
        self[0].object_is(&other[0]) && self[1].object_is(&other[1])
    }
}

impl ObjectIs for String {
    fn object_is(&self, other: &Self) -> bool {
        self == other
    }
}

impl ObjectIs for bool {
    fn object_is(&self, other: &Self) -> bool {
        self == other
    }
}

/// `Number(text)` for query values that must be finite: `None` is
/// math-viz2's `undefined` (the default). Empty or blank text parses as 0
/// (`Number("") === 0`), unsigned `0x` hex as its value (a sign makes it
/// unparseable: `Number("-0x10")` is `NaN`), `Infinity` and `NaN` (and
/// anything else unparseable) as `None`. Known gap: `0b`/`0o` literals fall
/// back to the default (math-viz2 parses them); queries never carry them.
pub fn parse_js_number(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Some(0.0);
    }
    let signed = trimmed.starts_with('-') || trimmed.starts_with('+');
    if !signed
        && let Some(hex) = trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
    {
        let magnitude = u128::from_str_radix(hex, 16).ok()? as f64;
        return magnitude.is_finite().then_some(magnitude);
    }
    let value: f64 = trimmed.parse().ok()?;
    value.is_finite().then_some(value)
}

/// Removes starting, trailing and duplicate slashes
/// (`router-util.ts`'s `trimSlashes`, whose regex is
/// `/^([/]*)|[/](?=[/])|([/]*)$/g`). Without a regex crate: dropping every
/// empty segment of a `/`-split removes exactly what each alternative
/// matches (leading runs, slashes followed by a slash, trailing runs) and
/// keeps the rest — the test cases pin it.
pub fn trim_slashes(route: &str) -> String {
    let mut out = String::new();
    for part in route.split('/').filter(|part| !part.is_empty()) {
        if !out.is_empty() {
            out.push('/');
        }
        out.push_str(part);
    }
    out
}

/// Splits a page hash into its route and search string
/// (`getRouteAndSearchString` over the `window.location.hash` argument
/// rather than the global: a leading `#` is cut, the split is at the first
/// `?`). Slashes are not trimmed here, as there.
pub fn get_route_and_search(hash: &str) -> (String, String) {
    let without_mark = hash.strip_prefix('#').unwrap_or(hash);
    match without_mark.split_once('?') {
        Some((route, search)) => (route.to_string(), search.to_string()),
        None => (without_mark.to_string(), String::new()),
    }
}

/// Joins a route and a search string into a page hash (`createFullHash`).
pub fn create_full_hash(route: &str, search: &str) -> String {
    if search.is_empty() {
        route.to_string()
    } else {
        format!("{route}?{search}")
    }
}

/// The query's pairs, decoded (`URLSearchParams` order: insertion, first
/// occurrence wins on read, `set` keeps position). A leading `?` is cut, as
/// the constructor cuts it; empty segments are skipped, as it skips them; a
/// segment without `=` reads as the empty value.
fn split_pairs(query: &str) -> Vec<(String, String)> {
    let query = query.strip_prefix('?').unwrap_or(query);
    let mut pairs = Vec::new();
    for part in query.split('&') {
        if part.is_empty() {
            continue;
        }
        let (name, value) = match part.split_once('=') {
            Some((name, value)) => (name, value),
            None => (part, ""),
        };
        pairs.push((decode_query_part(name), decode_query_part(value)));
    }
    pairs
}

/// The first pair's decoded value for `key` (`URLSearchParams.get`).
fn raw_value(query: &str, key: &str) -> Option<String> {
    split_pairs(query)
        .into_iter()
        .find(|(name, _)| name.as_str() == key)
        .map(|(_, value)| value)
}

/// Merges one key into the serialised query: set keeps the first
/// occurrence's position (appends when absent) and drops any later
/// duplicates, as `URLSearchParams.set` collapses them; delete drops every
/// occurrence. Everything re-serialises and literal commas come back (the
/// `replaceAll(/%2C/gi, ",")` with its Safe comment: a literal `%2C` in
/// input decodes to a comma and encodes back through `%2C`, so it can never
/// arrive bare).
fn write_pair(query: &str, key: &str, value: Option<String>) -> String {
    let mut pairs = split_pairs(query);
    match value {
        Some(text) => {
            if let Some(slot) = pairs.iter_mut().find(|(name, _)| name.as_str() == key) {
                slot.1 = text;
                // `URLSearchParams.set` leaves one entry: the first keeps its
                // position (set above, order kept), the rest go.
                let mut kept = false;
                pairs.retain(|(name, _)| {
                    if name.as_str() != key {
                        return true;
                    }
                    if kept {
                        return false;
                    }
                    kept = true;
                    true
                });
            } else {
                pairs.push((key.to_string(), text));
            }
        }
        None => pairs.retain(|(name, _)| name.as_str() != key),
    }
    let mut search = String::new();
    for (index, (name, value)) in pairs.iter().enumerate() {
        if index > 0 {
            search.push('&');
        }
        search.push_str(&encode_query_part(name));
        search.push('=');
        search.push_str(&encode_query_part(value));
    }
    restore_commas(&search)
}

/// Decodes one query name or value: `+` is a space, `%XX` the byte (a stray
/// `%` stays literal, as `URLSearchParams` leaves it), lossy UTF-8 (its
/// lone surrogates read as the replacement character, as there).
fn decode_query_part(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'+' {
            out.push(b' ');
            index += 1;
        } else if bytes[index] == b'%'
            && bytes.get(index + 1).is_some()
            && bytes.get(index + 2).is_some()
        {
            match hex_byte(bytes[index + 1], bytes[index + 2]) {
                Some(byte) => {
                    out.push(byte);
                    index += 3;
                }
                None => {
                    out.push(b'%');
                    index += 1;
                }
            }
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Two hex digits to their byte, `None` unless both are hex.
fn hex_byte(hi: u8, lo: u8) -> Option<u8> {
    let hi = hex_value(hi)?;
    let lo = hex_value(lo)?;
    Some(hi << 4 | lo)
}

/// One hex digit's value, `None` for anything else.
fn hex_value(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

/// Encodes one query name or value as `URLSearchParams.toString` does
/// (`application/x-www-form-urlencoded`: alphanumerics and `* - . _` stay,
/// space is `+`, everything else is `%UPPERHEX` — including `~`, `!`, `'`,
/// `(` and `)`, unlike `encodeURIComponent`).
fn encode_query_part(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'*' | b'-' | b'.' | b'_' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(hex_char(byte >> 4));
                out.push(hex_char(byte & 15));
            }
        }
    }
    out
}

/// One uppercase hex digit.
fn hex_char(nibble: u8) -> char {
    (match nibble {
        0..=9 => b'0' + nibble,
        _ => b'A' + nibble - 10,
    }) as char
}

/// Restores literal commas after serialisation: `URLSearchParams` encodes
/// commas as `%2C`, while math-viz2 keeps them literal for readable URLs
/// (`center=1,2`, not `center=1%2C2`). Case-insensitive, as the TS regex is.
/// The search string is encoder output, so pure ASCII by construction.
fn restore_commas(search: &str) -> String {
    let bytes = search.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && matches!(bytes.get(index + 1), Some(b'2'))
            && matches!(bytes.get(index + 2), Some(b'C' | b'c'))
        {
            out.push(',');
            index += 3;
        } else {
            out.push(bytes[index] as char);
            index += 1;
        }
    }
    out
}

#[cfg(test)]
mod url_tests {
    use super::*;
    use mv_numfmt::round_to;

    /// The vitest `numberDef`: `toFixed(2)`, default 0. Serialises through
    /// `round_to` first: Rust's `{:.2}` rounds ties to even while `toFixed`
    /// rounds them up, so the tie direction is settled in decimal before
    /// printing (the workspace idiom); the printed `-` is ASCII, as
    /// `toFixed` prints (`format_number`'s U+2212 minus is for display,
    /// never the URL).
    const fn number_def(key: &'static str, debounce_ms: u64) -> QueryParamDef<f64> {
        QueryParamDef {
            key,
            default: 0.0,
            debounce_ms,
            parse: parse_number_opt,
            serialize: serialize_fixed2,
            equals: default_equals,
        }
    }

    fn parse_number_opt(raw: Option<&str>) -> Option<f64> {
        parse_js_number(raw?)
    }

    // The def type keeps `Option` because a serialise may delete the key
    // (TS `null`); these test defs never do, hence the allow.
    #[allow(clippy::unnecessary_wraps)]
    fn serialize_fixed2(value: &f64) -> Option<String> {
        Some(format!("{:.2}", round_to(*value, 2)))
    }

    fn parse_center_opt(raw: Option<&str>) -> Option<[f64; 2]> {
        let (first, second) = raw?.split_once(',')?;
        Some([parse_js_number(first)?, parse_js_number(second)?])
    }

    // Same `Option` shape as above: the def type allows key deletion.
    #[allow(clippy::unnecessary_wraps)]
    fn serialize_center(value: &[f64; 2]) -> Option<String> {
        Some(format!(
            "{:.4},{:.4}",
            round_to(value[0], 4),
            round_to(value[1], 4)
        ))
    }

    const fn center_def(key: &'static str, debounce_ms: u64) -> QueryParamDef<[f64; 2]> {
        QueryParamDef {
            key,
            default: [0.0, 0.0],
            debounce_ms,
            parse: parse_center_opt,
            serialize: serialize_center,
            equals: default_equals,
        }
    }

    #[test]
    fn reads_the_value_from_the_url() {
        static DEF: QueryParamDef<f64> = number_def("qp-init", 20);
        let url = Url::new("#?qp-init=3.5");
        let param = QueryParam::new(&DEF, url.query());
        assert_eq!(*param.get(), 3.5);
    }

    #[test]
    fn falls_back_to_the_default_when_the_key_is_missing() {
        static DEF: QueryParamDef<f64> = number_def("qp-missing", 20);
        let url = Url::new("");
        let param = QueryParam::new(&DEF, url.query());
        assert_eq!(*param.get(), 0.0);
    }

    #[test]
    fn falls_back_to_the_default_when_the_value_is_invalid() {
        static DEF: QueryParamDef<f64> = number_def("qp-invalid", 20);
        let url = Url::new("#?qp-invalid=nope");
        let param = QueryParam::new(&DEF, url.query());
        assert_eq!(*param.get(), 0.0);
    }

    #[test]
    fn updates_live_immediately_without_touching_the_url() {
        static DEF: QueryParamDef<f64> = number_def("qp-live", 20);
        let mut url = Url::new("");
        let mut param = QueryParam::new(&DEF, url.query());
        url.set(&mut param, 5.0, 0);
        assert_eq!(url.get(&param), 5.0);
        assert_eq!(url.query(), "");
    }

    #[test]
    fn supports_updater_functions() {
        // No updater closure crosses into Rust: the caller reads `get`,
        // computes, and `set`s — the same live transition.
        static DEF: QueryParamDef<f64> = number_def("qp-updater", 20);
        let mut url = Url::new("#?qp-updater=2");
        let mut param = QueryParam::new(&DEF, url.query());
        let next = *param.get() + 1.0;
        url.set(&mut param, next, 0);
        assert_eq!(url.get(&param), 3.0);
    }

    #[test]
    fn writes_the_value_to_the_url_after_the_debounce() {
        static DEF: QueryParamDef<f64> = number_def("qp-debounce", 20);
        let mut url = Url::new("");
        let mut param = QueryParam::new(&DEF, url.query());
        url.set(&mut param, 5.0, 0);
        assert!(!param.step(&mut url, 19));
        assert_eq!(url.query(), "");
        assert!(param.step(&mut url, 20));
        assert!(url.query().contains("qp-debounce=5.00"));
    }

    #[test]
    fn writes_without_adding_a_history_entry() {
        // No SDL history API exists to assert on: the write path is
        // `history.replaceState` by construction (`mv_write_hash`), which
        // accrues no entry, while a location assignment would. The flushed
        // write lands in the query at once.
        static DEF: QueryParamDef<f64> = number_def("qp-history", 20);
        let mut url = Url::new("");
        let mut param = QueryParam::new(&DEF, url.query());
        param.set_flushed(&mut url, 5.0);
        assert!(url.query().contains("qp-history=5.00"));
    }

    #[test]
    fn writes_immediately_with_the_flush_option() {
        static DEF: QueryParamDef<f64> = number_def("qp-flush", 20);
        let mut url = Url::new("");
        let mut param = QueryParam::new(&DEF, url.query());
        param.set_flushed(&mut url, 5.0);
        assert!(url.query().contains("qp-flush=5.00"));
    }

    #[test]
    fn removes_the_key_when_the_value_equals_the_default() {
        static DEF: QueryParamDef<f64> = number_def("qp-prune", 20);
        let mut url = Url::new("#?qp-prune=5");
        let mut param = QueryParam::new(&DEF, url.query());
        assert_eq!(*param.get(), 5.0);
        param.set_flushed(&mut url, 0.0);
        assert_eq!(*param.get(), 0.0);
        assert_eq!(url.query(), "");
    }

    #[test]
    fn keeps_sub_serialize_ticks_in_live_state() {
        static DEF: QueryParamDef<f64> = number_def("qp-momentum", 20);
        let url = Url::new("");
        let mut param = QueryParam::new(&DEF, url.query());
        for _ in 0..10 {
            let next = *param.get() + 0.001;
            param.set(next, 0);
        }
        // `toBeCloseTo(0.01, 10)`: within half a unit of the 10th decimal.
        assert!((*param.get() - 0.01).abs() < 0.5e-10);
        // No step ran, so the debounced write never landed.
        assert_eq!(url.query(), "");
    }

    #[test]
    fn flows_a_hash_edit_into_the_live_value() {
        static DEF: QueryParamDef<f64> = number_def("qp-external", 20);
        let mut url = Url::new("");
        let mut param = QueryParam::new(&DEF, url.query());
        assert_eq!(*param.get(), 0.0);
        // The `hashchange` path: fresh hash in, then each param syncs.
        url.sync_from_hash("#?qp-external=7");
        assert!(param.sync(&url));
        assert_eq!(*param.get(), 7.0);
    }

    #[test]
    fn keeps_a_zoom_when_a_pan_writes_the_url_before_the_zoom_does() {
        static SCALE: QueryParamDef<f64> = number_def("qp-scale", 20);
        static PAN: QueryParamDef<f64> = number_def("qp-pan", 20);
        let mut url = Url::new("#?qp-scale=1");
        let mut scale = QueryParam::new(&SCALE, url.query());
        let mut pan = QueryParam::new(&PAN, url.query());
        // The zoom to 2: live at once, the URL still saying 1.
        scale.set(2.0, 0);
        // The pan ends and writes now, while the URL still says scale 1.
        pan.set_flushed(&mut url, 3.0);
        assert!(url.query().contains("qp-scale=1"));
        // The zoom's key text did not change, so the sync skips and the
        // live 2 survives the URL's 1.
        assert_eq!(*scale.get(), 2.0);
        assert!(!scale.sync(&url));
        assert_eq!(*scale.get(), 2.0);
        // The debounce fires: both keys land.
        assert!(scale.step(&mut url, 50));
        assert!(url.query().contains("qp-scale=2.00"));
        assert!(url.query().contains("qp-pan=3.00"));
    }

    #[test]
    fn round_trips_comma_separated_values_with_literal_commas() {
        static DEF: QueryParamDef<[f64; 2]> = center_def("qp-center", 20);
        let mut url = Url::new("#?qp-center=1.2325,5");
        let mut param = QueryParam::new(&DEF, url.query());
        assert_eq!(*param.get(), [1.2325, 5.0]);
        param.set_flushed(&mut url, [2.0, 3.0]);
        assert!(url.query().contains("qp-center=2.0000,3.0000"));
    }

    #[test]
    fn trim_slashes() {
        let cases = [
            ("", ""),
            ("hello/there", "hello/there"),
            ("/", ""),
            ("//", ""),
            ("///////", ""),
            ("/hello", "hello"),
            ("hello/", "hello"),
            ("/hello/", "hello"),
            ("////hello/////", "hello"),
            ("////hello/there/////", "hello/there"),
            ("////hello//there/////", "hello/there"),
            ("////hello///there/////", "hello/there"),
            ("/////hello//////there/////", "hello/there"),
        ];
        for (route, expected) in cases {
            assert_eq!(super::trim_slashes(route), expected, "route {route:?}");
        }
    }

    #[test]
    fn get_route_and_search_string() {
        let cases = [
            ("", ("", "")),
            ("#", ("", "")),
            ("#/", ("/", "")),
            ("#hello-there", ("hello-there", "")),
            ("?search", ("", "search")),
            ("#?search", ("", "search")),
            (
                "#sub-dir/page-1?search=hi&param2=val2",
                ("sub-dir/page-1", "search=hi&param2=val2"),
            ),
        ];
        for (hash, (route, search)) in cases {
            assert_eq!(
                super::get_route_and_search(hash),
                (route.to_string(), search.to_string()),
                "hash {hash:?}"
            );
        }
    }

    #[test]
    fn create_full_hash() {
        let cases = [
            ("", "", ""),
            ("", "param1=hello", "?param1=hello"),
            ("page-1", "", "page-1"),
            ("page-1", "param1=hello", "page-1?param1=hello"),
        ];
        for (route, search, expected) in cases {
            assert_eq!(super::create_full_hash(route, search), expected);
        }
    }

    #[test]
    fn default_equals_follows_object_is() {
        // `Object.is(-0, 0)` is false: a `set(-0.0)` over a live `0.0`
        // re-arms the debounce, as the TypeScript's early return skips it.
        assert!(default_equals(&0.0, &0.0));
        assert!(!default_equals(&-0.0, &0.0));
        assert!(!default_equals(&0.0, &-0.0));
        assert!(default_equals(&-0.0, &-0.0));
        // `Object.is(NaN, NaN)` is true (params are finite by parse, so this
        // only shows the rule is whole).
        assert!(default_equals(&f64::NAN, &f64::NAN));
        // Arrays compare element-wise, as the TypeScript's array branch does.
        assert!(default_equals(&[1.0, -0.0], &[1.0, -0.0]));
        assert!(!default_equals(&[1.0, 0.0], &[1.0, -0.0]));
        assert!(!default_equals(&[1.0, 2.0], &[1.0, 3.0]));
    }

    #[test]
    fn write_merges_and_keeps_order() {
        let mut url = Url::from_query("a=1&b=2");
        url.write("b", Some("3".to_string()));
        assert_eq!(url.query(), "a=1&b=3");
        url.write("c", Some("4".to_string()));
        assert_eq!(url.query(), "a=1&b=3&c=4");
        url.write("a", None);
        assert_eq!(url.query(), "b=3&c=4");
        url.write("missing", None);
        assert_eq!(url.query(), "b=3&c=4");
    }

    #[test]
    fn write_set_collapses_duplicate_keys() {
        // `URLSearchParams.set` leaves one entry: the first keeps its
        // position with the new value, later duplicates go.
        let mut url = Url::from_query("a=1&a=2&b=3");
        url.write("a", Some("9".to_string()));
        assert_eq!(url.query(), "a=9&b=3");
        // Delete still drops every occurrence.
        let mut url = Url::from_query("a=1&a=2&b=3");
        url.write("a", None);
        assert_eq!(url.query(), "b=3");
    }

    #[test]
    fn comma_restore_after_reencode() {
        // A re-serialised query keeps literal commas (never `%2C`), and a
        // value holding a space round-trips through `+`.
        let mut url = Url::from_query("qp-center=2.0000,3.0000&label=a+b");
        url.write("other", Some("x,y".to_string()));
        assert_eq!(url.query(), "qp-center=2.0000,3.0000&label=a+b&other=x,y");
        assert_eq!(url.raw("label"), Some("a b".to_string()));
    }

    #[test]
    fn parse_js_number_cases() {
        assert_eq!(parse_js_number("3.5"), Some(3.5));
        assert_eq!(parse_js_number("nope"), None);
        assert_eq!(parse_js_number(""), Some(0.0));
        assert_eq!(parse_js_number("  "), Some(0.0));
        assert_eq!(parse_js_number("0x10"), Some(16.0));
        assert_eq!(parse_js_number("-0x10"), None);
        assert_eq!(parse_js_number("+0x10"), None);
        assert_eq!(parse_js_number("Infinity"), None);
        assert_eq!(parse_js_number("1e3"), Some(1000.0));
    }
}
