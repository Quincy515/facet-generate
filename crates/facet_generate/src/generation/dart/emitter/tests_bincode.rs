//! Snapshot tests for the Dart emitter — **Bincode encoding**.
//!
//! Mirrors the structure of [`tests`](super::tests) but uses
//! `Encoding::Bincode` so that every generated type includes hand-written
//! `serialize`/`deserialize` methods using the `Serializer`/`Deserializer`
//! interface pattern.
//!
//! These tests verify the byte-level serialization code that the emitter
//! produces: field ordering, nested `serialize`/`deserialize` calls for
//! user-defined types, collection helpers (`serializeArray`, `deserializeMap`,
//! etc.), option encoding (`serializeOption`/`deserializeOption`), and the
//! `Serializer`/`Deserializer` API surface.

#![allow(clippy::too_many_lines)]
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use crate as fg;
use facet::Facet;

use super::*;
use crate::emit;

#[test]
fn unit_struct_1() {
    #[derive(Facet)]
    struct UnitStruct;

    let actual = emit!(UnitStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class UnitStruct {
        const UnitStruct();

        void bincodeEncode(BincodeWriter w) {
        }

        static UnitStruct bincodeDecode(BincodeReader r) {
            return const UnitStruct();
        }
    }
    ");
}

#[test]
fn unit_struct_2() {
    #[derive(Facet)]
    struct UnitStruct {}

    let actual = emit!(UnitStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class UnitStruct {
        const UnitStruct();

        void bincodeEncode(BincodeWriter w) {
        }

        static UnitStruct bincodeDecode(BincodeReader r) {
            return const UnitStruct();
        }
    }
    ");
}

#[test]
fn newtype_struct() {
    #[derive(Facet)]
    struct NewType(String);

    let actual = emit!(NewType as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class NewType {
        final String value;

        const NewType(this.value);

        void bincodeEncode(BincodeWriter w) {
            w.writeString(value);
        }

        static NewType bincodeDecode(BincodeReader r) {
            final value = r.readString();
            return NewType(value);
        }
    }
    ");
}

#[test]
fn tuple_struct() {
    #[derive(Facet)]
    struct TupleStruct(String, i32);

    let actual = emit!(TupleStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class TupleStruct {
        final String field0;
        final int field1;

        const TupleStruct(this.field0, this.field1);

        void bincodeEncode(BincodeWriter w) {
            w.writeString(field0);
            w.writeI32(field1);
        }

        static TupleStruct bincodeDecode(BincodeReader r) {
            final field0 = r.readString();
            final field1 = r.readI32();
            return TupleStruct(field0, field1);
        }
    }
    ");
}

#[test]
fn struct_with_fields_of_primitive_types() {
    #[derive(Facet)]
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

    let actual = emit!(StructWithFields as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class StructWithFields {
        final Null unit;
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

        void bincodeEncode(BincodeWriter w) {
            // unit: no bytes
            w.writeBool(bool);
            w.writeI8(i8);
            w.writeI16(i16);
            w.writeI32(i32);
            w.writeI64(i64);
            w.writeI128(i128);
            w.writeU8(u8);
            w.writeU16(u16);
            w.writeU32(u32);
            w.writeU64(u64);
            w.writeU128(u128);
            w.writeF32(f32);
            w.writeF64(f64);
            w.writeChar(char);
            w.writeString(string);
        }

        static StructWithFields bincodeDecode(BincodeReader r) {
            final unit = null;
            final bool = r.readBool();
            final i8 = r.readI8();
            final i16 = r.readI16();
            final i32 = r.readI32();
            final i64 = r.readI64();
            final i128 = r.readI128();
            final u8 = r.readU8();
            final u16 = r.readU16();
            final u32 = r.readU32();
            final u64 = r.readU64();
            final u128 = r.readU128();
            final f32 = r.readF32();
            final f64 = r.readF64();
            final char = r.readChar();
            final string = r.readString();
            return StructWithFields(unit: unit, bool: bool, i8: i8, i16: i16, i32: i32, i64: i64, i128: i128, u8: u8, u16: u16, u32: u32, u64: u64, u128: u128, f32: f32, f64: f64, char: char, string: string);
        }
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

    let actual = emit!(Outer as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class Inner1 {
        final String field1;

        const Inner1({required this.field1});

        void bincodeEncode(BincodeWriter w) {
            w.writeString(field1);
        }

        static Inner1 bincodeDecode(BincodeReader r) {
            final field1 = r.readString();
            return Inner1(field1: field1);
        }
    }


    final class Inner2 {
        final String value;

        const Inner2(this.value);

        void bincodeEncode(BincodeWriter w) {
            w.writeString(value);
        }

        static Inner2 bincodeDecode(BincodeReader r) {
            final value = r.readString();
            return Inner2(value);
        }
    }


    final class Inner3 {
        final String field0;
        final int field1;

        const Inner3(this.field0, this.field1);

        void bincodeEncode(BincodeWriter w) {
            w.writeString(field0);
            w.writeI32(field1);
        }

        static Inner3 bincodeDecode(BincodeReader r) {
            final field0 = r.readString();
            final field1 = r.readI32();
            return Inner3(field0, field1);
        }
    }


    final class Outer {
        final Inner1 one;
        final Inner2 two;
        final Inner3 three;

        const Outer({required this.one, required this.two, required this.three});

        void bincodeEncode(BincodeWriter w) {
            one.bincodeEncode(w);
            two.bincodeEncode(w);
            three.bincodeEncode(w);
        }

        static Outer bincodeDecode(BincodeReader r) {
            final one = Inner1.bincodeDecode(r);
            final two = Inner2.bincodeDecode(r);
            final three = Inner3.bincodeDecode(r);
            return Outer(one: one, two: two, three: three);
        }
    }
    ");
}

#[test]
fn struct_with_field_that_is_a_2_tuple() {
    #[derive(Facet)]
    struct MyStruct {
        one: (String, i32),
    }

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<dynamic> one;

        const MyStruct({required this.one});

        void bincodeEncode(BincodeWriter w) {
            w.writeString((one[0]));
            w.writeI32((one[1]));
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _one_t0 = r.readString();
            final _one_t1 = r.readI32();
            final one = <dynamic>[_one_t0, _one_t1];
            return MyStruct(one: one);
        }
    }
    ");
}

#[test]
fn struct_with_field_that_is_a_3_tuple() {
    #[derive(Facet)]
    struct MyStruct {
        one: (String, i32, u16),
    }

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<dynamic> one;

        const MyStruct({required this.one});

        void bincodeEncode(BincodeWriter w) {
            w.writeString((one[0]));
            w.writeI32((one[1]));
            w.writeU16((one[2]));
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _one_t0 = r.readString();
            final _one_t1 = r.readI32();
            final _one_t2 = r.readU16();
            final one = <dynamic>[_one_t0, _one_t1, _one_t2];
            return MyStruct(one: one);
        }
    }
    ");
}

#[test]
fn struct_with_field_that_is_a_4_tuple() {
    #[derive(Facet)]
    struct MyStruct {
        one: (String, i32, u16, f32),
    }

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<dynamic> one;

        const MyStruct({required this.one});

        void bincodeEncode(BincodeWriter w) {
            w.writeString((one[0]));
            w.writeI32((one[1]));
            w.writeU16((one[2]));
            w.writeF32((one[3]));
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _one_t0 = r.readString();
            final _one_t1 = r.readI32();
            final _one_t2 = r.readU16();
            final _one_t3 = r.readF32();
            final one = <dynamic>[_one_t0, _one_t1, _one_t2, _one_t3];
            return MyStruct(one: one);
        }
    }
    ");
}

#[test]
fn enum_with_unit_variants() {
    #[derive(Facet)]
    #[repr(C)]
    #[allow(unused)]
    enum EnumWithUnitVariants {
        Variant1,
        Variant2,
        Variant3,
    }

    let actual = emit!(EnumWithUnitVariants as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    enum EnumWithUnitVariants {
        Variant1, Variant2, Variant3;

        void bincodeEncode(BincodeWriter w) {
            w.writeU32(index);
        }

        static EnumWithUnitVariants bincodeDecode(BincodeReader r) {
            final idx = r.readU32();
            if (idx < 0 || idx >= values.length) {
                throw StateError('Unknown EnumWithUnitVariants variant index: $idx');
            }
            return values[idx];
        }
    }
    ");
}

#[test]
fn enum_with_unit_struct_variants() {
    #[derive(Facet)]
    #[repr(C)]
    #[allow(unused)]
    enum MyEnum {
        // Dart has the same emitted shape for unit and unit-struct variants.
        Variant1 {},
    }

    let actual = emit!(MyEnum as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    enum MyEnum {
        Variant1;

        void bincodeEncode(BincodeWriter w) {
            w.writeU32(index);
        }

        static MyEnum bincodeDecode(BincodeReader r) {
            final idx = r.readU32();
            if (idx < 0 || idx >= values.length) {
                throw StateError('Unknown MyEnum variant index: $idx');
            }
            return values[idx];
        }
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

    let actual = emit!(MyEnum as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class MyEnum {
        const MyEnum();

        void bincodeEncode(BincodeWriter w);

        static MyEnum bincodeDecode(BincodeReader r) {
            final variant = r.readU32();
            switch (variant) {
                case 0: {
                    final value = r.readString();
                    return MyEnumVariantVariant1(value);
                }
                default: throw StateError('Unknown MyEnum variant: $variant');
            }
        }
    }

    final class MyEnumVariantVariant1 extends MyEnum {
        final String value;

        const MyEnumVariantVariant1(this.value);

        @override
        void bincodeEncode(BincodeWriter w) {
            w.writeU32(0);
            w.writeString(this.value);
        }
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

    let actual = emit!(MyEnum as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class MyEnum {
        const MyEnum();

        void bincodeEncode(BincodeWriter w);

        static MyEnum bincodeDecode(BincodeReader r) {
            final variant = r.readU32();
            switch (variant) {
                case 0: {
                    final value = r.readString();
                    return MyEnumVariantVariant1(value);
                }
                case 1: {
                    final value = r.readI32();
                    return MyEnumVariantVariant2(value);
                }
                default: throw StateError('Unknown MyEnum variant: $variant');
            }
        }
    }

    final class MyEnumVariantVariant1 extends MyEnum {
        final String value;

        const MyEnumVariantVariant1(this.value);

        @override
        void bincodeEncode(BincodeWriter w) {
            w.writeU32(0);
            w.writeString(this.value);
        }
    }

    final class MyEnumVariantVariant2 extends MyEnum {
        final int value;

        const MyEnumVariantVariant2(this.value);

        @override
        void bincodeEncode(BincodeWriter w) {
            w.writeU32(1);
            w.writeI32(this.value);
        }
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

    let actual = emit!(MyEnum as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class MyEnum {
        const MyEnum();

        void bincodeEncode(BincodeWriter w);

        static MyEnum bincodeDecode(BincodeReader r) {
            final variant = r.readU32();
            switch (variant) {
                case 0: {
                    final field0 = r.readString();
                    final field1 = r.readI32();
                    return MyEnumVariantVariant1(field0, field1);
                }
                case 1: {
                    final field0 = r.readBool();
                    final field1 = r.readF64();
                    final field2 = r.readU8();
                    return MyEnumVariantVariant2(field0, field1, field2);
                }
                default: throw StateError('Unknown MyEnum variant: $variant');
            }
        }
    }

    final class MyEnumVariantVariant1 extends MyEnum {
        final String field0;
        final int field1;

        const MyEnumVariantVariant1(this.field0, this.field1);

        @override
        void bincodeEncode(BincodeWriter w) {
            w.writeU32(0);
            w.writeString(this.field0);
            w.writeI32(this.field1);
        }
    }

    final class MyEnumVariantVariant2 extends MyEnum {
        final bool field0;
        final double field1;
        final int field2;

        const MyEnumVariantVariant2(this.field0, this.field1, this.field2);

        @override
        void bincodeEncode(BincodeWriter w) {
            w.writeU32(1);
            w.writeBool(this.field0);
            w.writeF64(this.field1);
            w.writeU8(this.field2);
        }
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

    let actual = emit!(MyEnum as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class MyEnum {
        const MyEnum();

        void bincodeEncode(BincodeWriter w);

        static MyEnum bincodeDecode(BincodeReader r) {
            final variant = r.readU32();
            switch (variant) {
                case 0: {
                    final field1 = r.readString();
                    final field2 = r.readI32();
                    return MyEnumVariantVariant1(field1: field1, field2: field2);
                }
                default: throw StateError('Unknown MyEnum variant: $variant');
            }
        }
    }

    final class MyEnumVariantVariant1 extends MyEnum {
        final String field1;
        final int field2;

        const MyEnumVariantVariant1({required this.field1, required this.field2});

        @override
        void bincodeEncode(BincodeWriter w) {
            w.writeU32(0);
            w.writeString(this.field1);
            w.writeI32(this.field2);
        }
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

    let actual = emit!(MyEnum as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    sealed class MyEnum {
        const MyEnum();

        void bincodeEncode(BincodeWriter w);

        static MyEnum bincodeDecode(BincodeReader r) {
            final variant = r.readU32();
            switch (variant) {
                case 0: return const MyEnumVariantUnit();
                case 1: {
                    final value = r.readString();
                    return MyEnumVariantNewType(value);
                }
                case 2: {
                    final field0 = r.readString();
                    final field1 = r.readI32();
                    return MyEnumVariantTuple(field0, field1);
                }
                case 3: {
                    final field = r.readBool();
                    return MyEnumVariantStruct(field: field);
                }
                default: throw StateError('Unknown MyEnum variant: $variant');
            }
        }
    }

    final class MyEnumVariantUnit extends MyEnum {
        const MyEnumVariantUnit();

        @override
        void bincodeEncode(BincodeWriter w) {
            w.writeU32(0);
        }
    }

    final class MyEnumVariantNewType extends MyEnum {
        final String value;

        const MyEnumVariantNewType(this.value);

        @override
        void bincodeEncode(BincodeWriter w) {
            w.writeU32(1);
            w.writeString(this.value);
        }
    }

    final class MyEnumVariantTuple extends MyEnum {
        final String field0;
        final int field1;

        const MyEnumVariantTuple(this.field0, this.field1);

        @override
        void bincodeEncode(BincodeWriter w) {
            w.writeU32(2);
            w.writeString(this.field0);
            w.writeI32(this.field1);
        }
    }

    final class MyEnumVariantStruct extends MyEnum {
        final bool field;

        const MyEnumVariantStruct({required this.field});

        @override
        void bincodeEncode(BincodeWriter w) {
            w.writeU32(3);
            w.writeBool(this.field);
        }
    }
    ");
}

#[test]
fn struct_with_vec_field_1() {
    #[derive(Facet)]
    struct MyStruct {
        items: Vec<String>,
        numbers: Vec<i32>,
        nested_items: Vec<Vec<String>>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<String> items;
        final List<int> numbers;
        final List<List<String>> nested_items;

        const MyStruct({required this.items, required this.numbers, required this.nested_items});

        void bincodeEncode(BincodeWriter w) {
            w.writeU64(items.length);
            for (final _item in items) {
            w.writeString(_item);
            }
            w.writeU64(numbers.length);
            for (final _item in numbers) {
            w.writeI32(_item);
            }
            w.writeU64(nested_items.length);
            for (final _item in nested_items) {
            w.writeU64(_item.length);
            for (final _item in _item) {
            w.writeString(_item);
            }
            }
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _items_len = r.readU64();
            final items = List.generate(_items_len, (_) => r.readString());
            final _numbers_len = r.readU64();
            final numbers = List.generate(_numbers_len, (_) => r.readI32());
            final _nested_items_len = r.readU64();
            final nested_items = List.generate(_nested_items_len, (_) => (() { final _l = r.readU64(); return List.generate(_l, (_) => r.readString()); })());
            return MyStruct(items: items, numbers: numbers, nested_items: nested_items);
        }
    }
    ");
}

#[test]
fn struct_with_vec_field_2() {
    #[derive(Facet)]
    pub struct Child {
        name: String,
    }

    #[derive(Facet)]
    pub struct Parent {
        children: Vec<Vec<Child>>,
    }

    let actual = emit!(Parent as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class Child {
        final String name;

        const Child({required this.name});

        void bincodeEncode(BincodeWriter w) {
            w.writeString(name);
        }

        static Child bincodeDecode(BincodeReader r) {
            final name = r.readString();
            return Child(name: name);
        }
    }


    final class Parent {
        final List<List<Child>> children;

        const Parent({required this.children});

        void bincodeEncode(BincodeWriter w) {
            w.writeU64(children.length);
            for (final _item in children) {
            w.writeU64(_item.length);
            for (final _item in _item) {
            _item.bincodeEncode(w);
            }
            }
        }

        static Parent bincodeDecode(BincodeReader r) {
            final _children_len = r.readU64();
            final children = List.generate(_children_len, (_) => (() { final _l = r.readU64(); return List.generate(_l, (_) => Child.bincodeDecode(r)); })());
            return Parent(children: children);
        }
    }
    ");
}

#[test]
fn struct_with_option_field() {
    // English: `nested: Option<Option<i32>>` removed for Dart backend — Dart's
    // `T?` cannot represent all 3 distinct values of Rust nested Option
    // (None / Some(None) / Some(Some(v))). The emitter now panics on nested
    // Option to prevent silently-broken code; users should use a custom
    // wrapper struct if they need this.
    // 中文:`nested: Option<Option<i32>>` 字段从 Dart 后端的测试中移除 ——
    // Dart 的 `T?` 无法表达 Rust 嵌套 Option 的 3 种值。emitter 现在会在
    // 遇到嵌套 Option 时 panic,避免生成静默错误的代码;用户应该用自定义
    // 包装结构体处理这种情况。
    #[derive(Facet)]
    #[allow(clippy::struct_field_names)]
    struct MyStruct {
        simple: Option<String>,
        list: Option<Vec<bool>>,
        list_of_options: Vec<Option<bool>>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final String? simple;
        final List<bool>? list;
        final List<bool?> list_of_options;

        const MyStruct({required this.simple, required this.list, required this.list_of_options});

        void bincodeEncode(BincodeWriter w) {
            if (simple != null) {
                w.writeU8(1);
            w.writeString(simple!);
            } else {
                w.writeU8(0);
            }
            if (list != null) {
                w.writeU8(1);
            w.writeU64(list!.length);
            for (final _item in list!) {
            w.writeBool(_item);
            }
            } else {
                w.writeU8(0);
            }
            w.writeU64(list_of_options.length);
            for (final _item in list_of_options) {
            if (_item != null) {
                w.writeU8(1);
            w.writeBool(_item!);
            } else {
                w.writeU8(0);
            }
            }
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _simple_tag = r.readU8();
            final simple = _simple_tag == 1
                ? r.readString()
                : null;
            final _list_tag = r.readU8();
            final list = _list_tag == 1
                ? (() { final _l = r.readU64(); return List.generate(_l, (_) => r.readBool()); })()
                : null;
            final _list_of_options_len = r.readU64();
            final list_of_options = List.generate(_list_of_options_len, (_) => (() { final _t = r.readU8(); return _t == 1 ? r.readBool() : null; })());
            return MyStruct(simple: simple, list: list, list_of_options: list_of_options);
        }
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

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final Map<String, int> string_to_int;
        final Map<int, bool> int_to_bool;

        const MyStruct({required this.string_to_int, required this.int_to_bool});

        void bincodeEncode(BincodeWriter w) {
            w.writeU64(string_to_int.length);
            for (final _entry in string_to_int.entries) {
            w.writeString(_entry.key);
            w.writeI32(_entry.value);
            }
            w.writeU64(int_to_bool.length);
            for (final _entry in int_to_bool.entries) {
            w.writeI32(_entry.key);
            w.writeBool(_entry.value);
            }
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _string_to_int_len = r.readU64();
            final string_to_int = {
                for (var _i = 0; _i < _string_to_int_len; _i++)
                    r.readString(): r.readI32(),
            };
            final _int_to_bool_len = r.readU64();
            final int_to_bool = {
                for (var _i = 0; _i < _int_to_bool_len; _i++)
                    r.readI32(): r.readBool(),
            };
            return MyStruct(string_to_int: string_to_int, int_to_bool: int_to_bool);
        }
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

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<String>? optional_list;
        final List<int?> list_of_optionals;
        final Map<String, List<bool>> map_to_list;
        final Map<String, int>? optional_map;
        final List<Map<String, List<bool>>?> complex;

        const MyStruct({required this.optional_list, required this.list_of_optionals, required this.map_to_list, required this.optional_map, required this.complex});

        void bincodeEncode(BincodeWriter w) {
            if (optional_list != null) {
                w.writeU8(1);
            w.writeU64(optional_list!.length);
            for (final _item in optional_list!) {
            w.writeString(_item);
            }
            } else {
                w.writeU8(0);
            }
            w.writeU64(list_of_optionals.length);
            for (final _item in list_of_optionals) {
            if (_item != null) {
                w.writeU8(1);
            w.writeI32(_item!);
            } else {
                w.writeU8(0);
            }
            }
            w.writeU64(map_to_list.length);
            for (final _entry in map_to_list.entries) {
            w.writeString(_entry.key);
            w.writeU64(_entry.value.length);
            for (final _item in _entry.value) {
            w.writeBool(_item);
            }
            }
            if (optional_map != null) {
                w.writeU8(1);
            w.writeU64(optional_map!.length);
            for (final _entry in optional_map!.entries) {
            w.writeString(_entry.key);
            w.writeI32(_entry.value);
            }
            } else {
                w.writeU8(0);
            }
            w.writeU64(complex.length);
            for (final _item in complex) {
            if (_item != null) {
                w.writeU8(1);
            w.writeU64(_item!.length);
            for (final _entry in _item!.entries) {
            w.writeString(_entry.key);
            w.writeU64(_entry.value.length);
            for (final _item in _entry.value) {
            w.writeBool(_item);
            }
            }
            } else {
                w.writeU8(0);
            }
            }
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _optional_list_tag = r.readU8();
            final optional_list = _optional_list_tag == 1
                ? (() { final _l = r.readU64(); return List.generate(_l, (_) => r.readString()); })()
                : null;
            final _list_of_optionals_len = r.readU64();
            final list_of_optionals = List.generate(_list_of_optionals_len, (_) => (() { final _t = r.readU8(); return _t == 1 ? r.readI32() : null; })());
            final _map_to_list_len = r.readU64();
            final map_to_list = {
                for (var _i = 0; _i < _map_to_list_len; _i++)
                    r.readString(): (() { final _l = r.readU64(); return List.generate(_l, (_) => r.readBool()); })(),
            };
            final _optional_map_tag = r.readU8();
            final optional_map = _optional_map_tag == 1
                ? (() { final _l = r.readU64(); return { for (var _i = 0; _i < _l; _i++) r.readString(): r.readI32() }; })()
                : null;
            final _complex_len = r.readU64();
            final complex = List.generate(_complex_len, (_) => (() { final _t = r.readU8(); return _t == 1 ? (() { final _l = r.readU64(); return { for (var _i = 0; _i < _l; _i++) r.readString(): (() { final _l = r.readU64(); return List.generate(_l, (_) => r.readBool()); })() }; })() : null; })());
            return MyStruct(optional_list: optional_list, list_of_optionals: list_of_optionals, map_to_list: map_to_list, optional_map: optional_map, complex: complex);
        }
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

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<int> fixed_array;
        final List<int> byte_array;
        final List<String> string_array;

        const MyStruct({required this.fixed_array, required this.byte_array, required this.string_array});

        void bincodeEncode(BincodeWriter w) {
            // TupleArray: fixed size 5, no length prefix
            for (var _i = 0; _i < 5; _i++) { final _item = fixed_array[_i];
            w.writeI32(_item);
            }
            // TupleArray: fixed size 32, no length prefix
            for (var _i = 0; _i < 32; _i++) { final _item = byte_array[_i];
            w.writeU8(_item);
            }
            // TupleArray: fixed size 3, no length prefix
            for (var _i = 0; _i < 3; _i++) { final _item = string_array[_i];
            w.writeString(_item);
            }
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final fixed_array = List.generate(5, (_) => r.readI32());
            final byte_array = List.generate(32, (_) => r.readU8());
            final string_array = List.generate(3, (_) => r.readString());
            return MyStruct(fixed_array: fixed_array, byte_array: byte_array, string_array: string_array);
        }
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

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final Map<String, int> string_to_int;
        final Map<int, bool> int_to_bool;

        const MyStruct({required this.string_to_int, required this.int_to_bool});

        void bincodeEncode(BincodeWriter w) {
            w.writeU64(string_to_int.length);
            for (final _entry in string_to_int.entries) {
            w.writeString(_entry.key);
            w.writeI32(_entry.value);
            }
            w.writeU64(int_to_bool.length);
            for (final _entry in int_to_bool.entries) {
            w.writeI32(_entry.key);
            w.writeBool(_entry.value);
            }
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _string_to_int_len = r.readU64();
            final string_to_int = {
                for (var _i = 0; _i < _string_to_int_len; _i++)
                    r.readString(): r.readI32(),
            };
            final _int_to_bool_len = r.readU64();
            final int_to_bool = {
                for (var _i = 0; _i < _int_to_bool_len; _i++)
                    r.readI32(): r.readBool(),
            };
            return MyStruct(string_to_int: string_to_int, int_to_bool: int_to_bool);
        }
    }
    ");
}

#[test]
fn struct_with_nested_map_field() {
    #[derive(Facet)]
    struct MyStruct {
        map_to_list: HashMap<String, Vec<i32>>,
        list_to_map: Vec<HashMap<i32, String>>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final Map<String, List<int>> map_to_list;
        final List<Map<int, String>> list_to_map;

        const MyStruct({required this.map_to_list, required this.list_to_map});

        void bincodeEncode(BincodeWriter w) {
            w.writeU64(map_to_list.length);
            for (final _entry in map_to_list.entries) {
            w.writeString(_entry.key);
            w.writeU64(_entry.value.length);
            for (final _item in _entry.value) {
            w.writeI32(_item);
            }
            }
            w.writeU64(list_to_map.length);
            for (final _item in list_to_map) {
            w.writeU64(_item.length);
            for (final _entry in _item.entries) {
            w.writeI32(_entry.key);
            w.writeString(_entry.value);
            }
            }
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _map_to_list_len = r.readU64();
            final map_to_list = {
                for (var _i = 0; _i < _map_to_list_len; _i++)
                    r.readString(): (() { final _l = r.readU64(); return List.generate(_l, (_) => r.readI32()); })(),
            };
            final _list_to_map_len = r.readU64();
            final list_to_map = List.generate(_list_to_map_len, (_) => (() { final _l = r.readU64(); return { for (var _i = 0; _i < _l; _i++) r.readI32(): r.readString() }; })());
            return MyStruct(map_to_list: map_to_list, list_to_map: list_to_map);
        }
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

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<String> string_set;
        final List<int> int_set;

        const MyStruct({required this.string_set, required this.int_set});

        void bincodeEncode(BincodeWriter w) {
            w.writeU64(string_set.length);
            for (final _item in string_set) {
            w.writeString(_item);
            }
            w.writeU64(int_set.length);
            for (final _item in int_set) {
            w.writeI32(_item);
            }
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _string_set_len = r.readU64();
            final string_set = List.generate(_string_set_len, (_) => r.readString());
            final _int_set_len = r.readU64();
            final int_set = List.generate(_int_set_len, (_) => r.readI32());
            return MyStruct(string_set: string_set, int_set: int_set);
        }
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

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<String> string_set;
        final List<int> int_set;

        const MyStruct({required this.string_set, required this.int_set});

        void bincodeEncode(BincodeWriter w) {
            w.writeU64(string_set.length);
            for (final _item in string_set) {
            w.writeString(_item);
            }
            w.writeU64(int_set.length);
            for (final _item in int_set) {
            w.writeI32(_item);
            }
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _string_set_len = r.readU64();
            final string_set = List.generate(_string_set_len, (_) => r.readString());
            final _int_set_len = r.readU64();
            final int_set = List.generate(_int_set_len, (_) => r.readI32());
            return MyStruct(string_set: string_set, int_set: int_set);
        }
    }
    ");
}

#[test]
fn struct_with_nested_set_field() {
    #[derive(Facet)]
    struct MyStruct {
        vec_of_sets: Vec<HashSet<String>>,
        set_of_ints: HashSet<i32>,
    }

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<List<String>> vec_of_sets;
        final List<int> set_of_ints;

        const MyStruct({required this.vec_of_sets, required this.set_of_ints});

        void bincodeEncode(BincodeWriter w) {
            w.writeU64(vec_of_sets.length);
            for (final _item in vec_of_sets) {
            w.writeU64(_item.length);
            for (final _item in _item) {
            w.writeString(_item);
            }
            }
            w.writeU64(set_of_ints.length);
            for (final _item in set_of_ints) {
            w.writeI32(_item);
            }
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _vec_of_sets_len = r.readU64();
            final vec_of_sets = List.generate(_vec_of_sets_len, (_) => (() { final _l = r.readU64(); return List.generate(_l, (_) => r.readString()); })());
            final _set_of_ints_len = r.readU64();
            final set_of_ints = List.generate(_set_of_ints_len, (_) => r.readI32());
            return MyStruct(vec_of_sets: vec_of_sets, set_of_ints: set_of_ints);
        }
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

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final String boxed_string;
        final int boxed_int;

        const MyStruct({required this.boxed_string, required this.boxed_int});

        void bincodeEncode(BincodeWriter w) {
            w.writeString(boxed_string);
            w.writeI32(boxed_int);
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final boxed_string = r.readString();
            final boxed_int = r.readI32();
            return MyStruct(boxed_string: boxed_string, boxed_int: boxed_int);
        }
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

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final String rc_string;
        final int rc_int;

        const MyStruct({required this.rc_string, required this.rc_int});

        void bincodeEncode(BincodeWriter w) {
            w.writeString(rc_string);
            w.writeI32(rc_int);
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final rc_string = r.readString();
            final rc_int = r.readI32();
            return MyStruct(rc_string: rc_string, rc_int: rc_int);
        }
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

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final String arc_string;
        final int arc_int;

        const MyStruct({required this.arc_string, required this.arc_int});

        void bincodeEncode(BincodeWriter w) {
            w.writeString(arc_string);
            w.writeI32(arc_int);
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final arc_string = r.readString();
            final arc_int = r.readI32();
            return MyStruct(arc_string: arc_string, arc_int: arc_int);
        }
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

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final List<List<String>> vec_of_sets;
        final Map<String, int>? optional_btree;
        final List<String> boxed_vec;
        final String? arc_option;
        final List<int> array_of_boxes;

        const MyStruct({required this.vec_of_sets, required this.optional_btree, required this.boxed_vec, required this.arc_option, required this.array_of_boxes});

        void bincodeEncode(BincodeWriter w) {
            w.writeU64(vec_of_sets.length);
            for (final _item in vec_of_sets) {
            w.writeU64(_item.length);
            for (final _item in _item) {
            w.writeString(_item);
            }
            }
            if (optional_btree != null) {
                w.writeU8(1);
            w.writeU64(optional_btree!.length);
            for (final _entry in optional_btree!.entries) {
            w.writeString(_entry.key);
            w.writeI32(_entry.value);
            }
            } else {
                w.writeU8(0);
            }
            w.writeU64(boxed_vec.length);
            for (final _item in boxed_vec) {
            w.writeString(_item);
            }
            if (arc_option != null) {
                w.writeU8(1);
            w.writeString(arc_option!);
            } else {
                w.writeU8(0);
            }
            // TupleArray: fixed size 3, no length prefix
            for (var _i = 0; _i < 3; _i++) { final _item = array_of_boxes[_i];
            w.writeI32(_item);
            }
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final _vec_of_sets_len = r.readU64();
            final vec_of_sets = List.generate(_vec_of_sets_len, (_) => (() { final _l = r.readU64(); return List.generate(_l, (_) => r.readString()); })());
            final _optional_btree_tag = r.readU8();
            final optional_btree = _optional_btree_tag == 1
                ? (() { final _l = r.readU64(); return { for (var _i = 0; _i < _l; _i++) r.readString(): r.readI32() }; })()
                : null;
            final _boxed_vec_len = r.readU64();
            final boxed_vec = List.generate(_boxed_vec_len, (_) => r.readString());
            final _arc_option_tag = r.readU8();
            final arc_option = _arc_option_tag == 1
                ? r.readString()
                : null;
            final array_of_boxes = List.generate(3, (_) => r.readI32());
            return MyStruct(vec_of_sets: vec_of_sets, optional_btree: optional_btree, boxed_vec: boxed_vec, arc_option: arc_option, array_of_boxes: array_of_boxes);
        }
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

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final Uint8List data;
        final String name;
        final Uint8List header;

        const MyStruct({required this.data, required this.name, required this.header});

        void bincodeEncode(BincodeWriter w) {
            w.writeU64(data.length);
            w.writeBytes(data);
            w.writeString(name);
            w.writeU64(header.length);
            w.writeBytes(header);
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final data = (() { final _l = r.readU64(); return r.readBytes(_l); })();
            final name = r.readString();
            final header = (() { final _l = r.readU64(); return r.readBytes(_l); })();
            return MyStruct(data: data, name: name, header: header);
        }
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

    let actual = emit!(MyStruct as Dart with Encoding::Bincode).unwrap();
    insta::assert_snapshot!(actual, @"


    final class MyStruct {
        final Uint8List data;
        final String name;
        final Uint8List header;
        final List<int>? optional_bytes;

        const MyStruct({required this.data, required this.name, required this.header, required this.optional_bytes});

        void bincodeEncode(BincodeWriter w) {
            w.writeU64(data.length);
            w.writeBytes(data);
            w.writeString(name);
            w.writeU64(header.length);
            w.writeBytes(header);
            if (optional_bytes != null) {
                w.writeU8(1);
            w.writeU64(optional_bytes!.length);
            for (final _item in optional_bytes!) {
            w.writeU8(_item);
            }
            } else {
                w.writeU8(0);
            }
        }

        static MyStruct bincodeDecode(BincodeReader r) {
            final data = (() { final _l = r.readU64(); return r.readBytes(_l); })();
            final name = r.readString();
            final header = (() { final _l = r.readU64(); return r.readBytes(_l); })();
            final _optional_bytes_tag = r.readU8();
            final optional_bytes = _optional_bytes_tag == 1
                ? (() { final _l = r.readU64(); return List.generate(_l, (_) => r.readU8()); })()
                : null;
            return MyStruct(data: data, name: name, header: header, optional_bytes: optional_bytes);
        }
    }
    ");
}
