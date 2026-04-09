//! Snapshot tests for the Dart emitter — **JSON encoding**.
//!
//! Mirrors the structure of [`tests`](super::tests) and
//! [`tests_bincode`](super::tests_bincode) but uses `Encoding::Json` so that
//! every generated type includes `toJson` / `fromJson` methods matching
//! serde's default externally tagged JSON wire format.
//!
//! These tests verify the JSON serialization code that the emitter produces:
//! - field-name keys in `toJson` output
//! - native `Map<String, dynamic>` / `List<dynamic>` literals
//! - `Option<T>` mapped to JSON `null` / value (no explicit tag)
//! - externally tagged enums: unit variants → bare strings, payload variants
//!   → single-key map with the variant name as key
//! - native Dart `enum` for all-unit variants (using `.name` property)
//! - sealed class with `Object toJson()` and `factory fromJson(Object)`
//!   that dispatches on JSON shape (String for unit, Map for payload)

#![allow(clippy::too_many_lines)]
use std::collections::{BTreeMap, HashMap};

use facet::Facet;

use super::*;
use crate::emit;

// ─── Struct cases ────────────────────────────────────────────────────────

#[test]
fn unit_struct_json() {
    /// line 1
    #[derive(Facet)]
    /// line 2
    struct UnitStruct;

    let actual = emit!(UnitStruct as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    /// line 1
    /// line 2
    final class UnitStruct {
        const UnitStruct();

        Map<String, dynamic> toJson() {
            return {
            };
        }

        factory UnitStruct.fromJson(Map<String, dynamic> json) {
            return const UnitStruct();
        }
    }
    ");
}

#[test]
fn newtype_struct_json() {
    #[derive(Facet)]
    #[allow(dead_code)]
    struct Wrapper(String);

    let actual = emit!(Wrapper as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    final class Wrapper {
        final String value;

        const Wrapper(this.value);

        Map<String, dynamic> toJson() {
            return {
                'value': value,
            };
        }

        factory Wrapper.fromJson(Map<String, dynamic> json) {
            final value = json['value'] as String;
            return Wrapper(value);
        }
    }
    ");
}

#[test]
fn tuple_struct_json() {
    #[derive(Facet)]
    #[allow(dead_code)]
    struct Pair(String, i32);

    let actual = emit!(Pair as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    final class Pair {
        final String field0;
        final int field1;

        const Pair(this.field0, this.field1);

        Map<String, dynamic> toJson() {
            return {
                'field0': field0,
                'field1': field1,
            };
        }

        factory Pair.fromJson(Map<String, dynamic> json) {
            final field0 = json['field0'] as String;
            final field1 = (json['field1'] as num).toInt();
            return Pair(field0, field1);
        }
    }
    ");
}

#[test]
fn struct_with_primitive_fields_json() {
    #[derive(Facet)]
    #[allow(dead_code)]
    struct ViewModel {
        text: String,
        confirmed: bool,
        platform: String,
        count: u32,
        ratio: f64,
    }

    let actual = emit!(ViewModel as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    final class ViewModel {
        final String text;
        final bool confirmed;
        final String platform;
        final int count;
        final double ratio;

        const ViewModel({required this.text, required this.confirmed, required this.platform, required this.count, required this.ratio});

        Map<String, dynamic> toJson() {
            return {
                'text': text,
                'confirmed': confirmed,
                'platform': platform,
                'count': count,
                'ratio': ratio,
            };
        }

        factory ViewModel.fromJson(Map<String, dynamic> json) {
            final text = json['text'] as String;
            final confirmed = json['confirmed'] as bool;
            final platform = json['platform'] as String;
            final count = (json['count'] as num).toInt();
            final ratio = (json['ratio'] as num).toDouble();
            return ViewModel(text: text, confirmed: confirmed, platform: platform, count: count, ratio: ratio);
        }
    }
    ");
}

#[test]
fn struct_with_option_fields_json() {
    #[derive(Facet)]
    #[allow(dead_code, clippy::struct_field_names)]
    struct Maybe {
        maybe_name: Option<String>,
        maybe_count: Option<u32>,
        maybe_flag: Option<bool>,
    }

    let actual = emit!(Maybe as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    final class Maybe {
        final String? maybe_name;
        final int? maybe_count;
        final bool? maybe_flag;

        const Maybe({required this.maybe_name, required this.maybe_count, required this.maybe_flag});

        Map<String, dynamic> toJson() {
            return {
                'maybe_name': maybe_name,
                'maybe_count': maybe_count,
                'maybe_flag': maybe_flag,
            };
        }

        factory Maybe.fromJson(Map<String, dynamic> json) {
            final maybe_name = json['maybe_name'] == null ? null : (json['maybe_name'] as String);
            final maybe_count = json['maybe_count'] == null ? null : ((json['maybe_count'] as num).toInt());
            final maybe_flag = json['maybe_flag'] == null ? null : (json['maybe_flag'] as bool);
            return Maybe(maybe_name: maybe_name, maybe_count: maybe_count, maybe_flag: maybe_flag);
        }
    }
    ");
}

#[test]
fn struct_with_vec_fields_json() {
    #[derive(Facet)]
    #[allow(dead_code)]
    struct WithVec {
        items: Vec<String>,
        numbers: Vec<i32>,
    }

    let actual = emit!(WithVec as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    final class WithVec {
        final List<String> items;
        final List<int> numbers;

        const WithVec({required this.items, required this.numbers});

        Map<String, dynamic> toJson() {
            return {
                'items': items,
                'numbers': numbers,
            };
        }

        factory WithVec.fromJson(Map<String, dynamic> json) {
            final items = (json['items'] as List<dynamic>).map((_e) => _e as String).toList();
            final numbers = (json['numbers'] as List<dynamic>).map((_e) => (_e as num).toInt()).toList();
            return WithVec(items: items, numbers: numbers);
        }
    }
    ");
}

#[test]
fn struct_with_map_fields_json() {
    #[derive(Facet)]
    #[allow(dead_code)]
    struct WithMaps {
        string_to_int: HashMap<String, i32>,
        sorted: BTreeMap<String, bool>,
    }

    let actual = emit!(WithMaps as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    final class WithMaps {
        final Map<String, int> string_to_int;
        final Map<String, bool> sorted;

        const WithMaps({required this.string_to_int, required this.sorted});

        Map<String, dynamic> toJson() {
            return {
                'string_to_int': string_to_int,
                'sorted': sorted,
            };
        }

        factory WithMaps.fromJson(Map<String, dynamic> json) {
            final string_to_int = (json['string_to_int'] as Map<String, dynamic>).map((k, _v) => MapEntry(k, (_v as num).toInt()));
            final sorted = (json['sorted'] as Map<String, dynamic>).map((k, _v) => MapEntry(k, _v as bool));
            return WithMaps(string_to_int: string_to_int, sorted: sorted);
        }
    }
    ");
}

#[test]
fn struct_with_nested_user_type_json() {
    #[derive(Facet)]
    #[allow(dead_code)]
    struct Inner {
        value: String,
    }

    #[derive(Facet)]
    #[allow(dead_code)]
    struct Outer {
        inner: Inner,
        maybe_inner: Option<Inner>,
        list_of_inner: Vec<Inner>,
    }

    let actual = emit!(Outer as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    final class Inner {
        final String value;

        const Inner({required this.value});

        Map<String, dynamic> toJson() {
            return {
                'value': value,
            };
        }

        factory Inner.fromJson(Map<String, dynamic> json) {
            final value = json['value'] as String;
            return Inner(value: value);
        }
    }


    final class Outer {
        final Inner inner;
        final Inner? maybe_inner;
        final List<Inner> list_of_inner;

        const Outer({required this.inner, required this.maybe_inner, required this.list_of_inner});

        Map<String, dynamic> toJson() {
            return {
                'inner': inner.toJson(),
                'maybe_inner': maybe_inner == null ? null : maybe_inner!.toJson(),
                'list_of_inner': list_of_inner.map((e) => e.toJson()).toList(),
            };
        }

        factory Outer.fromJson(Map<String, dynamic> json) {
            final inner = Inner.fromJson(json['inner'] as Map<String, dynamic>);
            final maybe_inner = json['maybe_inner'] == null ? null : (Inner.fromJson(json['maybe_inner'] as Map<String, dynamic>));
            final list_of_inner = (json['list_of_inner'] as List<dynamic>).map((_e) => Inner.fromJson(_e as Map<String, dynamic>)).toList();
            return Outer(inner: inner, maybe_inner: maybe_inner, list_of_inner: list_of_inner);
        }
    }
    ");
}

// ─── Enum cases ──────────────────────────────────────────────────────────

#[test]
fn enum_with_only_unit_variants_json() {
    /// Doc line 1
    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum Event {
        None,
        Increment,
        Decrement,
        Reset,
    }

    let actual = emit!(Event as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    /// Doc line 1
    enum Event {
        None, Increment, Decrement, Reset;

        String toJson() {
            return name;
        }

        static Event fromJson(String s) {
            for (final v in values) {
                if (v.name == s) return v;
            }
            throw StateError('Unknown Event variant: $s');
        }
    }
    ");
}

#[test]
fn enum_with_newtype_variants_json() {
    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum Cmd {
        Quit,
        SetName(String),
        SetCount(i32),
    }

    let actual = emit!(Cmd as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class Cmd {
        const Cmd();

        Object toJson();

        factory Cmd.fromJson(Object json) {
            if (json is String) {
                switch (json) {
                    case 'Quit': return const CmdVariantQuit();
                    default: throw StateError('Unknown Cmd unit variant: $json');
                }
            }
            if (json is Map<String, dynamic>) {
                final entry = json.entries.single;
                switch (entry.key) {
                    case 'SetName': {
                        final value = entry.value as String;
                        return CmdVariantSetName(value);
                    }
                    case 'SetCount': {
                        final value = (entry.value as num).toInt();
                        return CmdVariantSetCount(value);
                    }
                    default: throw StateError('Unknown Cmd variant: ${entry.key}');
                }
            }
            throw FormatException('Unexpected Cmd JSON: $json');
        }
    }

    final class CmdVariantQuit extends Cmd {
        const CmdVariantQuit();

        @override
        Object toJson() {
            return 'Quit';
        }
    }

    final class CmdVariantSetName extends Cmd {
        final String value;

        const CmdVariantSetName(this.value);

        @override
        Object toJson() {
            return {'SetName': value};
        }
    }

    final class CmdVariantSetCount extends Cmd {
        final int value;

        const CmdVariantSetCount(this.value);

        @override
        Object toJson() {
            return {'SetCount': value};
        }
    }
    ");
}

#[test]
fn enum_with_tuple_variants_json() {
    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum Shape {
        Empty,
        Point(i32, i32),
        Triangle(i32, i32, i32),
    }

    let actual = emit!(Shape as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class Shape {
        const Shape();

        Object toJson();

        factory Shape.fromJson(Object json) {
            if (json is String) {
                switch (json) {
                    case 'Empty': return const ShapeVariantEmpty();
                    default: throw StateError('Unknown Shape unit variant: $json');
                }
            }
            if (json is Map<String, dynamic>) {
                final entry = json.entries.single;
                switch (entry.key) {
                    case 'Point': {
                        final inner = entry.value as List<dynamic>;
                        final field0 = (inner[0] as num).toInt();
                        final field1 = (inner[1] as num).toInt();
                        return ShapeVariantPoint(field0, field1);
                    }
                    case 'Triangle': {
                        final inner = entry.value as List<dynamic>;
                        final field0 = (inner[0] as num).toInt();
                        final field1 = (inner[1] as num).toInt();
                        final field2 = (inner[2] as num).toInt();
                        return ShapeVariantTriangle(field0, field1, field2);
                    }
                    default: throw StateError('Unknown Shape variant: ${entry.key}');
                }
            }
            throw FormatException('Unexpected Shape JSON: $json');
        }
    }

    final class ShapeVariantEmpty extends Shape {
        const ShapeVariantEmpty();

        @override
        Object toJson() {
            return 'Empty';
        }
    }

    final class ShapeVariantPoint extends Shape {
        final int field0;
        final int field1;

        const ShapeVariantPoint(this.field0, this.field1);

        @override
        Object toJson() {
            return {'Point': [field0, field1]};
        }
    }

    final class ShapeVariantTriangle extends Shape {
        final int field0;
        final int field1;
        final int field2;

        const ShapeVariantTriangle(this.field0, this.field1, this.field2);

        @override
        Object toJson() {
            return {'Triangle': [field0, field1, field2]};
        }
    }
    ");
}

#[test]
fn enum_with_struct_variants_json() {
    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum Action {
        Cancel,
        Move { x: i32, y: i32 },
        Rotate { degrees: f64 },
    }

    let actual = emit!(Action as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class Action {
        const Action();

        Object toJson();

        factory Action.fromJson(Object json) {
            if (json is String) {
                switch (json) {
                    case 'Cancel': return const ActionVariantCancel();
                    default: throw StateError('Unknown Action unit variant: $json');
                }
            }
            if (json is Map<String, dynamic>) {
                final entry = json.entries.single;
                switch (entry.key) {
                    case 'Move': {
                        final inner = entry.value as Map<String, dynamic>;
                        final x = (inner['x'] as num).toInt();
                        final y = (inner['y'] as num).toInt();
                        return ActionVariantMove(x: x, y: y);
                    }
                    case 'Rotate': {
                        final inner = entry.value as Map<String, dynamic>;
                        final degrees = (inner['degrees'] as num).toDouble();
                        return ActionVariantRotate(degrees: degrees);
                    }
                    default: throw StateError('Unknown Action variant: ${entry.key}');
                }
            }
            throw FormatException('Unexpected Action JSON: $json');
        }
    }

    final class ActionVariantCancel extends Action {
        const ActionVariantCancel();

        @override
        Object toJson() {
            return 'Cancel';
        }
    }

    final class ActionVariantMove extends Action {
        final int x;
        final int y;

        const ActionVariantMove({required this.x, required this.y});

        @override
        Object toJson() {
            return {'Move': {
                'x': x,
                'y': y,
            }};
        }
    }

    final class ActionVariantRotate extends Action {
        final double degrees;

        const ActionVariantRotate({required this.degrees});

        @override
        Object toJson() {
            return {'Rotate': {
                'degrees': degrees,
            }};
        }
    }
    ");
}

#[test]
fn enum_with_mixed_variants_json() {
    #[derive(Facet)]
    #[repr(C)]
    #[allow(dead_code)]
    enum Event {
        Quit,
        SetName(String),
        Move { x: i32, y: i32 },
        Position(i32, i32),
    }

    let actual = emit!(Event as Dart with Encoding::Json).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class Event {
        const Event();

        Object toJson();

        factory Event.fromJson(Object json) {
            if (json is String) {
                switch (json) {
                    case 'Quit': return const EventVariantQuit();
                    default: throw StateError('Unknown Event unit variant: $json');
                }
            }
            if (json is Map<String, dynamic>) {
                final entry = json.entries.single;
                switch (entry.key) {
                    case 'SetName': {
                        final value = entry.value as String;
                        return EventVariantSetName(value);
                    }
                    case 'Move': {
                        final inner = entry.value as Map<String, dynamic>;
                        final x = (inner['x'] as num).toInt();
                        final y = (inner['y'] as num).toInt();
                        return EventVariantMove(x: x, y: y);
                    }
                    case 'Position': {
                        final inner = entry.value as List<dynamic>;
                        final field0 = (inner[0] as num).toInt();
                        final field1 = (inner[1] as num).toInt();
                        return EventVariantPosition(field0, field1);
                    }
                    default: throw StateError('Unknown Event variant: ${entry.key}');
                }
            }
            throw FormatException('Unexpected Event JSON: $json');
        }
    }

    final class EventVariantQuit extends Event {
        const EventVariantQuit();

        @override
        Object toJson() {
            return 'Quit';
        }
    }

    final class EventVariantSetName extends Event {
        final String value;

        const EventVariantSetName(this.value);

        @override
        Object toJson() {
            return {'SetName': value};
        }
    }

    final class EventVariantMove extends Event {
        final int x;
        final int y;

        const EventVariantMove({required this.x, required this.y});

        @override
        Object toJson() {
            return {'Move': {
                'x': x,
                'y': y,
            }};
        }
    }

    final class EventVariantPosition extends Event {
        final int field0;
        final int field1;

        const EventVariantPosition(this.field0, this.field1);

        @override
        Object toJson() {
            return {'Position': [field0, field1]};
        }
    }
    ");
}
