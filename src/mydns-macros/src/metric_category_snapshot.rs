use proc_macro::TokenStream;
use quote::quote;
use syn::{Error, Fields, GenericArgument, ItemStruct, PathArguments, Type};

/// Expands the `metric_category_snapshot` attribute after validating its
/// structural contract.
///
/// A category snapshot must be a struct containing named
/// `start_time: DateTime<Utc>` and `end_time: DateTime<Utc>` fields.
/// The input struct is returned unchanged when the contract is satisfied.
pub fn expand(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as ItemStruct);

    match validate_times(&input) {
        Ok(()) => quote!(#input).into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn validate_times(input: &ItemStruct) -> Result<(), Error> {
    let Fields::Named(fields) = &input.fields else {
        return Err(Error::new_spanned(
            &input.ident,
            "metric category snapshot must be a struct with named fields",
        ));
    };

    validate_datetime_field(input, fields, "start_time")?;
    validate_datetime_field(input, fields, "end_time")?;

    Ok(())
}

fn validate_datetime_field(
    input: &ItemStruct,
    fields: &syn::FieldsNamed,
    field_name: &str,
) -> Result<(), Error> {
    let Some(field) = fields.named.iter().find(|field| {
        field
            .ident
            .as_ref()
            .is_some_and(|ident| ident == field_name)
    }) else {
        return Err(Error::new_spanned(
            &input.ident,
            format!(
                "metric category snapshot must contain a `{field_name}: DateTime<Utc>` field"
            ),
        ));
    };

    if !is_datetime_utc(&field.ty) {
        return Err(Error::new_spanned(
            &field.ty,
            format!(
                "metric category snapshot `{field_name}` must have type `DateTime<Utc>`"
            ),
        ));
    }

    Ok(())
}

fn is_datetime_utc(ty: &Type) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };

    let Some(datetime_segment) = type_path.path.segments.last() else {
        return false;
    };

    if datetime_segment.ident != "DateTime" {
        return false;
    }

    let PathArguments::AngleBracketed(arguments) = &datetime_segment.arguments else {
        return false;
    };

    let Some(GenericArgument::Type(Type::Path(utc_type))) = arguments.args.first() else {
        return false;
    };

    utc_type
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Utc")
}
