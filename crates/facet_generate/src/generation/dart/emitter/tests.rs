//! Snapshot tests for the Dart emitter — **no serialization**.
//!
//! Each test defines one or more Rust types annotated with `#[derive(Facet)]`,
//! runs them through the [`emit!`] macro with `Encoding::None`, and asserts the
//! generated Dart source against an [`insta`] inline snapshot.
//!
//! Because encoding is `None`, the output contains only plain type declarations
//! (`export class`, `export abstract class` + variant subclasses) with no
//! `serialize`/`deserialize` methods.
//!
//! # Coverage
//!
//! | Category | What is tested |
//! |----------|----------------|
//! | Structs | Unit structs (with/without body), newtype wrappers, tuple structs, structs with primitive and user-defined fields |
//! | Tuples | 2-tuple, 3-tuple, 4-tuple (via `Tuple<[…]>` type alias) |
//! | Enums | Unit variants, newtype/tuple variants, struct variants, mixed-variant enums |
//! | Collections | `Vec`, `HashMap`/`BTreeMap`, `HashSet`/`BTreeSet`, fixed-size arrays (`ListTuple`) |
//! | Optional | `Option<T>` fields (mapped to `Optional<T>`, i.e. `T \| null`) |
//! | Pointers | `Box`, `Rc`, `Arc` (all transparent in generated output) |
//! | Bytes | `#[facet(fg::bytes)]` fields (mapped to `Uint8Array` via `bytes` alias) |
//! | Modules | Cross-namespace references via `import * as Namespace` wildcard imports |

#![allow(clippy::too_many_lines)]
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use crate as fg;
use facet::Facet;

use super::*;
use crate::{emit, emit_two_modules, generation::dart::DartCodeGenerator};

// English: test_format_type_aliases removed — Dart doesn't use TYPE_ALIASES
// (native int/bool/double/String/List/Map/T? types cover everything).
// 中文:test_format_type_aliases 测试删除 —— Dart 不用 TYPE_ALIASES(原生类型
// 已经涵盖所有需求)。

#[test]
fn unit_struct() {
    /// line 1
    #[derive(Facet)]
    /// line 2
    struct UnitStruct;

    let actual = emit!(UnitStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    /// line 1
    /// line 2
    final class UnitStruct {
        const UnitStruct();
    }
    ");
}

#[test]
fn unit_struct_empty_body() {
    /// line 1
    #[derive(Facet)]
    /// line 2
    struct UnitStruct {}

    let actual = emit!(UnitStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    /// line 1
    /// line 2
    final class UnitStruct {
        const UnitStruct();
    }
    ");
}

#[test]
fn newtype_struct() {
    /// line 1
    #[derive(Facet)]
    /// line 2
    struct NewType(String);

    let actual = emit!(NewType as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    /// line 1
    /// line 2
    final class NewType {
        final String value;

        const NewType(this.value);
    }
    ");
}

#[test]
fn tuple_struct() {
    /// line 1
    #[derive(Facet)]
    /// line 2
    struct TupleStruct(String, i32);

    let actual = emit!(TupleStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    /// line 1
    /// line 2
    final class TupleStruct {
        final String field0;
        final int field1;

        const TupleStruct(this.field0, this.field1);
    }
    ");
}

#[test]
fn struct_with_fields_of_primitive_types() {
    /// line 1
    #[derive(Facet)]
    /// line 2
    struct StructWithFields {
        unit: (),
        bool: bool,
        i8: i8,
        i16: i16,
        i32: i32,
        i64: i64,
        i128: i128,
        u8: u8,
        u16: u16,
        u32: u32,
        u64: u64,
        u128: u128,
        f32: f32,
        f64: f64,
        char: char,
        string: String,
    }

    let actual = emit!(StructWithFields as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    /// line 1
    /// line 2
    final class StructWithFields {
        final Unit unit;
        final bool bool;
        final int i8;
        final int i16;
        final int i32;
        final int i64;
        final BigInt i128;
        final int u8;
        final int u16;
        final int u32;
        final int u64;
        final BigInt u128;
        final double f32;
        final double f64;
        final String char;
        final String string;

        const StructWithFields({required this.unit, required this.bool, required this.i8, required this.i16, required this.i32, required this.i64, required this.i128, required this.u8, required this.u16, required this.u32, required this.u64, required this.u128, required this.f32, required this.f64, required this.char, required this.string});
    }
    ");
}

#[test]
fn struct_with_fields_of_user_types() {
    #[derive(Facet)]
    struct Inner1 {
        field1: String,
    }

    #[derive(Facet)]
    struct Inner2(String);

    #[derive(Facet)]
    struct Inner3(String, i32);

    #[derive(Facet)]
    struct Outer {
        one: Inner1,
        two: Inner2,
        three: Inner3,
    }

    let actual = emit!(Outer as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class Inner1 {
        final String field1;

        const Inner1({required this.field1});
    }


    final class Inner2 {
        final String value;

        const Inner2(this.value);
    }


    final class Inner3 {
        final String field0;
        final int field1;

        const Inner3(this.field0, this.field1);
    }


    final class Outer {
        final Inner1 one;
        final Inner2 two;
        final Inner3 three;

        const Outer({required this.one, required this.two, required this.three});
    }
    ");
}

#[test]
fn struct_with_field_that_is_a_2_tuple() {
    #[derive(Facet)]
    struct MyStruct {
        one: (String, i32),
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<dynamic> one;

        const MyStruct({required this.one});
    }
    ");
}

#[test]
fn struct_with_field_that_is_a_3_tuple() {
    #[derive(Facet)]
    struct MyStruct {
        one: (String, i32, u16),
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<dynamic> one;

        const MyStruct({required this.one});
    }
    ");
}

#[test]
fn struct_with_field_that_is_a_4_tuple() {
    #[derive(Facet)]
    struct MyStruct {
        one: (String, i32, u16, f32),
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<dynamic> one;

        const MyStruct({required this.one});
    }
    ");
}

#[test]
fn enum_with_unit_variants() {
    /// line one
    #[derive(Facet)]
    #[repr(C)]
    /// line two
    #[allow(unused)]
    enum EnumWithUnitVariants {
        /// variant one
        Variant1,
        /// variant two
        Variant2,
        /// variant three
        Variant3,
    }

    let actual = emit!(EnumWithUnitVariants as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    /// line one
    /// line two
    enum EnumWithUnitVariants {
        Variant1, Variant2, Variant3;
    }
    ");
}

#[test]
fn enum_with_unit_struct_variants() {
    #[derive(Facet)]
    #[repr(C)]
    #[allow(unused)]
    enum MyEnum {
        // TS has no separate "unit struct variant" representation, so this maps
        // to the same empty variant class shape as a unit variant.
        Variant1 {},
    }

    let actual = emit!(MyEnum as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    enum MyEnum {
        Variant1;
    }
    ");
}

#[test]
fn enum_with_1_tuple_variants() {
    #[derive(Facet)]
    #[repr(C)]
    #[allow(unused)]
    enum MyEnum {
        Variant1(String),
    }

    let actual = emit!(MyEnum as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class MyEnum {
        const MyEnum();
    }

    final class MyEnumVariantVariant1 extends MyEnum {
        final String value;

        const MyEnumVariantVariant1(this.value);
    }
    ");
}

#[test]
fn enum_with_newtype_variants() {
    #[derive(Facet)]
    #[repr(C)]
    #[allow(unused)]
    enum MyEnum {
        Variant1(String),
        Variant2(i32),
    }

    let actual = emit!(MyEnum as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class MyEnum {
        const MyEnum();
    }

    final class MyEnumVariantVariant1 extends MyEnum {
        final String value;

        const MyEnumVariantVariant1(this.value);
    }

    final class MyEnumVariantVariant2 extends MyEnum {
        final int value;

        const MyEnumVariantVariant2(this.value);
    }
    ");
}

#[test]
fn enum_with_tuple_variants() {
    #[derive(Facet)]
    #[repr(C)]
    #[allow(unused)]
    enum MyEnum {
        Variant1(String, i32),
        Variant2(bool, f64, u8),
    }

    let actual = emit!(MyEnum as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class MyEnum {
        const MyEnum();
    }

    final class MyEnumVariantVariant1 extends MyEnum {
        final String field0;
        final int field1;

        const MyEnumVariantVariant1(this.field0, this.field1);
    }

    final class MyEnumVariantVariant2 extends MyEnum {
        final bool field0;
        final double field1;
        final int field2;

        const MyEnumVariantVariant2(this.field0, this.field1, this.field2);
    }
    ");
}

#[test]
fn enum_with_struct_variants() {
    #[derive(Facet)]
    #[repr(C)]
    #[allow(unused)]
    enum MyEnum {
        Variant1 { field1: String, field2: i32 },
    }

    let actual = emit!(MyEnum as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class MyEnum {
        const MyEnum();
    }

    final class MyEnumVariantVariant1 extends MyEnum {
        final String field1;
        final int field2;

        const MyEnumVariantVariant1({required this.field1, required this.field2});
    }
    ");
}

#[test]
fn enum_with_mixed_variants() {
    #[derive(Facet)]
    #[repr(C)]
    #[allow(unused)]
    enum MyEnum {
        Unit,
        NewType(String),
        Tuple(String, i32),
        Struct { field: bool },
    }

    let actual = emit!(MyEnum as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class MyEnum {
        const MyEnum();
    }

    final class MyEnumVariantUnit extends MyEnum {
        const MyEnumVariantUnit();
    }

    final class MyEnumVariantNewType extends MyEnum {
        final String value;

        const MyEnumVariantNewType(this.value);
    }

    final class MyEnumVariantTuple extends MyEnum {
        final String field0;
        final int field1;

        const MyEnumVariantTuple(this.field0, this.field1);
    }

    final class MyEnumVariantStruct extends MyEnum {
        final bool field;

        const MyEnumVariantStruct({required this.field});
    }
    ");
}

#[test]
fn struct_with_vec_field() {
    #[derive(Facet)]
    struct MyStruct {
        items: Vec<String>,
        numbers: Vec<i32>,
        nested_items: Vec<Vec<String>>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<String> items;
        final List<int> numbers;
        final List<List<String>> nested_items;

        const MyStruct({required this.items, required this.numbers, required this.nested_items});
    }
    ");
}

#[test]
fn struct_with_option_field() {
    #[derive(Facet)]
    #[allow(clippy::struct_field_names)]
    struct MyStruct {
        optional_string: Option<String>,
        optional_number: Option<i32>,
        optional_bool: Option<bool>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final String? optional_string;
        final int? optional_number;
        final bool? optional_bool;

        const MyStruct({required this.optional_string, required this.optional_number, required this.optional_bool});
    }
    ");
}

#[test]
fn struct_with_hashmap_field() {
    #[derive(Facet)]
    struct MyStruct {
        string_to_int: HashMap<String, i32>,
        int_to_bool: HashMap<i32, bool>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final Map<String, int> string_to_int;
        final Map<int, bool> int_to_bool;

        const MyStruct({required this.string_to_int, required this.int_to_bool});
    }
    ");
}

#[test]
fn struct_with_nested_generics() {
    #[derive(Facet)]
    struct MyStruct {
        optional_list: Option<Vec<String>>,
        list_of_optionals: Vec<Option<i32>>,
        map_to_list: HashMap<String, Vec<bool>>,
        optional_map: Option<HashMap<String, i32>>,
        complex: Vec<Option<HashMap<String, Vec<bool>>>>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<String>? optional_list;
        final List<int?> list_of_optionals;
        final Map<String, List<bool>> map_to_list;
        final Map<String, int>? optional_map;
        final List<Map<String, List<bool>>?> complex;

        const MyStruct({required this.optional_list, required this.list_of_optionals, required this.map_to_list, required this.optional_map, required this.complex});
    }
    ");
}

#[test]
fn struct_with_array_field() {
    #[derive(Facet)]
    #[allow(clippy::struct_field_names)]
    struct MyStruct {
        fixed_array: [i32; 5],
        byte_array: [u8; 32],
        string_array: [String; 3],
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<int> fixed_array;
        final List<int> byte_array;
        final List<String> string_array;

        const MyStruct({required this.fixed_array, required this.byte_array, required this.string_array});
    }
    ");
}

#[test]
fn struct_with_btreemap_field() {
    #[derive(Facet)]
    struct MyStruct {
        string_to_int: BTreeMap<String, i32>,
        int_to_bool: BTreeMap<i32, bool>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final Map<String, int> string_to_int;
        final Map<int, bool> int_to_bool;

        const MyStruct({required this.string_to_int, required this.int_to_bool});
    }
    ");
}

#[test]
fn struct_with_hashset_field() {
    #[derive(Facet)]
    struct MyStruct {
        string_set: HashSet<String>,
        int_set: HashSet<i32>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<String> string_set;
        final List<int> int_set;

        const MyStruct({required this.string_set, required this.int_set});
    }
    ");
}

#[test]
fn struct_with_btreeset_field() {
    #[derive(Facet)]
    struct MyStruct {
        string_set: BTreeSet<String>,
        int_set: BTreeSet<i32>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<String> string_set;
        final List<int> int_set;

        const MyStruct({required this.string_set, required this.int_set});
    }
    ");
}

#[test]
fn struct_with_box_field() {
    #[derive(Facet)]
    #[allow(clippy::box_collection)]
    struct MyStruct {
        boxed_string: Box<String>,
        boxed_int: Box<i32>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final String boxed_string;
        final int boxed_int;

        const MyStruct({required this.boxed_string, required this.boxed_int});
    }
    ");
}

#[test]
fn struct_with_rc_field() {
    #[derive(Facet)]
    struct MyStruct {
        rc_string: Rc<String>,
        rc_int: Rc<i32>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final String rc_string;
        final int rc_int;

        const MyStruct({required this.rc_string, required this.rc_int});
    }
    ");
}

#[test]
fn struct_with_arc_field() {
    #[derive(Facet)]
    struct MyStruct {
        arc_string: Arc<String>,
        arc_int: Arc<i32>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final String arc_string;
        final int arc_int;

        const MyStruct({required this.arc_string, required this.arc_int});
    }
    ");
}

#[test]
fn struct_with_mixed_collections_and_pointers() {
    #[derive(Facet)]
    #[allow(clippy::box_collection)]
    struct MyStruct {
        vec_of_sets: Vec<HashSet<String>>,
        optional_btree: Option<BTreeMap<String, i32>>,
        boxed_vec: Box<Vec<String>>,
        arc_option: Arc<Option<String>>,
        array_of_boxes: [Box<i32>; 3],
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<List<String>> vec_of_sets;
        final Map<String, int>? optional_btree;
        final List<String> boxed_vec;
        final String? arc_option;
        final List<int> array_of_boxes;

        const MyStruct({required this.vec_of_sets, required this.optional_btree, required this.boxed_vec, required this.arc_option, required this.array_of_boxes});
    }
    ");
}

#[test]
fn struct_with_bytes_field() {
    #[derive(Facet)]
    struct MyStruct {
        #[facet(fg::bytes)]
        data: Vec<u8>,
        name: String,
        #[facet(fg::bytes)]
        header: Vec<u8>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final Uint8List data;
        final String name;
        final Uint8List header;

        const MyStruct({required this.data, required this.name, required this.header});
    }
    ");
}

#[test]
fn struct_with_bytes_field_and_slice() {
    #[derive(Facet)]
    struct MyStruct {
        #[facet(fg::bytes)]
        data: &'static [u8],
        name: String,
        #[facet(fg::bytes)]
        header: Vec<u8>,
        optional_bytes: Option<Vec<u8>>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::None).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final Uint8List data;
        final String name;
        final Uint8List header;
        final List<int>? optional_bytes;

        const MyStruct({required this.data, required this.name, required this.header, required this.optional_bytes});
    }
    ");
}

#[test]
fn type_in_root_and_named_namespace() {
    #[derive(Facet)]
    struct Child {
        value: String,
    }

    mod other {
        use crate as fg;
        use facet::Facet;

        #[derive(Facet)]
        #[facet(fg::namespace = "other")]
        pub struct Child {
            value: i32,
        }
    }

    #[derive(Facet)]
    struct Parent {
        child: Child,
        other_child: other::Child,
    }

    let (other, root) = emit_two_modules!(DartCodeGenerator, Parent, "root");
    insta::assert_snapshot!(other, @"

    final class Child {
        final int value;

        const Child({required this.value});
    }
    ");
    insta::assert_snapshot!(root, @"
    import '../other.dart' as Other;

    final class Child {
        final String value;

        const Child({required this.value});
    }

    final class Parent {
        final Child child;
        final Other.Child other_child;

        const Parent({required this.child, required this.other_child});
    }
    ");
}
