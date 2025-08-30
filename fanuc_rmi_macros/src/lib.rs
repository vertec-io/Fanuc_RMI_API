use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Attribute, Data, DataEnum, DataStruct, DeriveInput, Fields, Ident, Type};

fn strip_serde_attrs(attrs: &[Attribute]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|a| {
            // keep all attributes except serde attributes that modify naming/tagging
            if !a.path().is_ident("serde") {
                return true;
            }
            // remove serde attributes entirely for DTOs
            false
        })
        .cloned()
        .collect()
}

fn dto_ident(original: &Ident) -> Ident {
    format_ident!("{}Dto", original)
}

fn is_primitive_ident(name: &str) -> bool {
    matches!(
        name,
        "bool"|
        "u8"|"u16"|"u32"|"u64"|"u128"|"usize"|
        "i8"|"i16"|"i32"|"i64"|"i128"|"isize"|
        "f32"|"f64"|
        "String"|"str"
    )
}

fn map_type_to_dto(ty: &mut Type) {
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last_mut() {
            let ident_str = seg.ident.to_string();
            if !is_primitive_ident(&ident_str) {
                if matches!(seg.arguments, syn::PathArguments::None) {
                    let dto_ident = format_ident!("{}Dto", seg.ident);
                    match ident_str.as_str() {
                        "Position" | "Configuration" | "FrameData" | "JointAngles" => {
                            *ty = syn::parse_quote!(crate::#dto_ident);
                        }
                        "OnOff" => {
                            *ty = syn::parse_quote!(crate::packets::#dto_ident);
                        }
                        // Keep these as-is (protocol enums reused in DTO)
                        "SpeedType" | "TermType" => {}
                        _ => {
                            seg.ident = dto_ident;
                        }
                    }
                }
            }
        }
    }
}

fn field_type_needs_into(ty: &Type) -> bool {
    // We request Into for Path types that are not primitives (likely mirrored)
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            return !is_primitive_ident(&seg.ident.to_string());
        }
    }
    false
}

#[proc_macro_attribute]
pub fn mirror_dto(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = input.ident.clone();
    let dto_name = dto_ident(&name);

    // Strip serde attrs on the mirror
    let vis = input.vis.clone();
    let generics = input.generics.clone();

    let dto_struct_or_enum = match &input.data {
        Data::Struct(data_struct) => mirror_struct(&name, &dto_name, &vis, &generics, data_struct, &input.attrs),
        Data::Enum(data_enum) => mirror_enum(&name, &dto_name, &vis, &generics, data_enum, &input.attrs),
        Data::Union(_) => {
            return syn::Error::new_spanned(&input, "mirror_dto does not support unions").to_compile_error().into();
        }
    };

    // Re-emit original item unchanged
    let original = quote! { #input };

    // Wrap generated DTO in feature gate and place conversions next to it
    let expanded = quote! {
        #original

        #[cfg(feature = "DTO")]
        #dto_struct_or_enum
    };

    expanded.into()
}

fn mirror_struct(
    original: &Ident,
    dto_name: &Ident,
    vis: &syn::Visibility,
    generics: &syn::Generics,
    data: &DataStruct,
    attrs: &[Attribute],
) -> proc_macro2::TokenStream {
    let _serde_stripped_attrs = strip_serde_attrs(attrs);

    let fields = match &data.fields {
        Fields::Named(named) => &named.named,
        _ => {
            return syn::Error::new_spanned(&data.fields, "mirror_dto requires named fields").to_compile_error();
        }
    };

    // Determine if fields likely need Into for nested DTOs
    let field_names: Vec<_> = fields.iter().map(|f| f.ident.clone().unwrap()).collect();
    let field_types: Vec<_> = fields.iter().map(|f| f.ty.clone()).collect();
    let nested_flags: Vec<_> = field_types.iter().map(|t| field_type_needs_into(t)).collect();

    let dto_fields = fields.iter().map(|f| {
        let mut f2 = f.clone();
        f2.attrs = strip_serde_attrs(&f.attrs);
        let mut ty = f2.ty.clone();
        map_type_to_dto(&mut ty);
        f2.ty = ty;
        quote! { #f2 }
    });

    let into_fields = field_names.iter().enumerate().map(|(i, name)| {
        if nested_flags[i] { quote! { #name: src.#name.into() } } else { quote! { #name: src.#name } }
    });

    let from_fields = field_names.iter().enumerate().map(|(i, name)| {
        if nested_flags[i] { quote! { #name: src.#name.into() } } else { quote! { #name: src.#name } }
    });

    quote! {
        #[derive(::serde::Serialize, ::serde::Deserialize, ::core::fmt::Debug, ::core::clone::Clone, ::core::cmp::PartialEq)]
        #vis struct #dto_name #generics { #( #dto_fields ),* }

        impl #generics ::core::convert::From<#original #generics> for #dto_name #generics {
            fn from(src: #original #generics) -> Self { Self { #( #into_fields ),* } }
        }
        impl #generics ::core::convert::From<#dto_name #generics> for #original #generics {
            fn from(src: #dto_name #generics) -> Self { Self { #( #from_fields ),* } }
        }
    }
}

fn mirror_enum(
    original: &Ident,
    dto_name: &Ident,
    vis: &syn::Visibility,
    generics: &syn::Generics,
    data: &DataEnum,
    attrs: &[Attribute],
) -> proc_macro2::TokenStream {
    let _serde_stripped_attrs2 = strip_serde_attrs(attrs);

    let mut dto_variants = Vec::new();
    let mut into_arms = Vec::new();
    let mut from_arms = Vec::new();

    for v in &data.variants {
        let v_name = &v.ident;
        let v_attrs = strip_serde_attrs(&v.attrs);
        match &v.fields {
            Fields::Unit => {
                dto_variants.push(quote! { #(#v_attrs)* #v_name });
                into_arms.push(quote! { #original::#v_name => #dto_name::#v_name });
                from_arms.push(quote! { #dto_name::#v_name => #original::#v_name });
            }
            Fields::Unnamed(unnamed) => {
                let field_idents: Vec<Ident> = (0..unnamed.unnamed.len()).map(|i| format_ident!("f{}", i)).collect();
                let field_types: Vec<Type> = unnamed.unnamed.iter().map(|f| f.ty.clone()).collect();
                let nested_flags: Vec<_> = field_types.iter().map(|t| field_type_needs_into(t)).collect();

                let dto_fields = unnamed.unnamed.iter().map(|f| {
                    let mut f2 = f.clone();
                    f2.attrs = strip_serde_attrs(&f.attrs);
                    let mut ty = f2.ty.clone();
                    map_type_to_dto_in_enum(&mut ty, original);
                    f2.ty = ty;
                    quote! { #f2 }
                });

                let into_exprs = field_idents.iter().enumerate().map(|(i, id)| if nested_flags[i] { quote! { #id.into() } } else { quote! { #id } });
                let from_exprs = field_idents.iter().enumerate().map(|(i, id)| if nested_flags[i] { quote! { #id.into() } } else { quote! { #id } });

                dto_variants.push(quote! { #(#v_attrs)* #v_name( #( #dto_fields ),* ) });
                into_arms.push(quote! { #original::#v_name( #( #field_idents ),* ) => #dto_name::#v_name( #( #into_exprs ),* ) });
                from_arms.push(quote! { #dto_name::#v_name( #( #field_idents ),* ) => #original::#v_name( #( #from_exprs ),* ) });
            }
            Fields::Named(named) => {
                let field_names: Vec<_> = named.named.iter().map(|f| f.ident.clone().unwrap()).collect();
                let field_types: Vec<_> = named.named.iter().map(|f| f.ty.clone()).collect();
                let nested_flags: Vec<_> = field_types.iter().map(|t| field_type_needs_into(t)).collect();

                let dto_fields = named.named.iter().map(|f| {
                    let mut f2 = f.clone();
                    f2.attrs = strip_serde_attrs(&f.attrs);
                    let mut ty = f2.ty.clone();
                    map_type_to_dto_in_enum(&mut ty, original);
                    f2.ty = ty;
                    quote! { #f2 }
                });

                let pat_bindings: Vec<Ident> = field_names.iter().map(|n| format_ident!("b_{}", n)).collect();
                let into_kvs = field_names.iter().enumerate().map(|(i, n)| {
                    let bind = &pat_bindings[i];
                    if nested_flags[i] { quote! { #n: #bind.into() } } else { quote! { #n: #bind } }
                });
                let from_kvs = field_names.iter().enumerate().map(|(i, n)| {
                    let bind = &pat_bindings[i];
                    if nested_flags[i] { quote! { #n: #bind.into() } } else { quote! { #n: #bind } }
                });

                dto_variants.push(quote! { #(#v_attrs)* #v_name { #( #dto_fields ),* } });
                into_arms.push(quote! { #original::#v_name { #( #field_names: #pat_bindings ),* } => #dto_name::#v_name { #( #into_kvs ),* } });
                from_arms.push(quote! { #dto_name::#v_name { #( #field_names: #pat_bindings ),* } => #original::#v_name { #( #from_kvs ),* } });
            }
        }
    }

    quote! {
        #[derive(::serde::Serialize, ::serde::Deserialize, ::core::fmt::Debug, ::core::clone::Clone, ::core::cmp::PartialEq)]
        #vis enum #dto_name #generics { #( #dto_variants ),* }

        impl #generics ::core::convert::From<#original #generics> for #dto_name #generics {
            fn from(src: #original #generics) -> Self { match src { #( #into_arms ),* } }
        }
        impl #generics ::core::convert::From<#dto_name #generics> for #original #generics {
            fn from(src: #dto_name #generics) -> Self { match src { #( #from_arms ),* } }
        }
    }
}

fn map_type_to_dto_in_enum(ty: &mut Type, enum_name: &Ident) {
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last_mut() {
            let ident_str = seg.ident.to_string();
            if !is_primitive_ident(&ident_str) {
                if matches!(seg.arguments, syn::PathArguments::None) {
                    let dto_ident = format_ident!("{}Dto", seg.ident);
                    match ident_str.as_str() {
                        // Core root types
                        "Position" | "Configuration" | "FrameData" | "JointAngles" => {
                            *ty = syn::parse_quote!(crate::#dto_ident);
                        }
                        // Packet-local enums
                        "OnOff" => {
                            *ty = syn::parse_quote!(crate::packets::#dto_ident);
                        }
                        // Keep these as-is (protocol enums reused in DTO)
                        "SpeedType" | "TermType" => {}
                        _ => {
                            let e = enum_name.to_string();
                            if e == "Instruction" || e == "InstructionResponse" {
                                // Use re-exported DTOs under crate::instructions::dto::<Original>
                                let base_ident = &seg.ident;
                                *ty = syn::parse_quote!(crate::instructions::dto::#base_ident);
                            } else if e == "Command" || e == "CommandResponse" {
                                let base_ident = &seg.ident;
                                *ty = syn::parse_quote!(crate::commands::dto::#base_ident);
                            } else {
                                // Fallback: just append Dto
                                seg.ident = dto_ident;
                            }
                        }
                    }
                }
            }
        }
    }
}

