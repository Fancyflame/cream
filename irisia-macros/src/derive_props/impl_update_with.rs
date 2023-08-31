use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{ItemStruct, Type};

use super::{
    attrs::{FieldDefault, FieldResolver},
    GenHelper, HandledField,
};

pub(super) fn impl_update_with(helper: &GenHelper) -> TokenStream {
    let update_with = generate_update_with(helper);
    let create_with = generate_create_with(helper);
    let where_clause = generate_where_clause(helper);
    let update_result = make_update_reuslt(helper);

    let GenHelper {
        item: ItemStruct {
            ident: origin_struct,
            ..
        },
        update_result_name,
        updater_name,
        updater_generics,
        ..
    } = helper;

    quote! {
        impl #updater_generics irisia::element::props::PropsUpdateWith<#updater_name #updater_generics>
            for #origin_struct
        where
            #where_clause
        {
            type UpdateResult = #update_result_name;

            #update_with
            #create_with
        }

        #update_result
    }
}

fn get_resolver(fr: &FieldResolver, field_type: &Type, use_expr: bool) -> TokenStream {
    match fr {
        FieldResolver::CallUpdater => quote!(irisia::element::props::CallUpdater),
        FieldResolver::Custom(custom) => quote!(#custom),
        FieldResolver::MoveOwnership => quote!(irisia::element::props::MoveOwnership),
        FieldResolver::ReadStyle { as_std_input: _ } => quote!(irisia::element::props::ReadStyle),
        FieldResolver::WithFn { arg_type, path } => {
            let ty = quote!(fn(#arg_type) -> #field_type);
            if use_expr {
                quote!((#path as #ty))
            } else {
                quote!(#ty)
            }
        }
    }
}

fn generate_where_clause(helper: &GenHelper) -> TokenStream {
    let mut output = quote!();
    for (field, generic_type) in helper.fields.iter().zip(helper.generics_iter()) {
        let field_type = field.ty;
        let resolver = get_resolver(&field.attr.value_resolver, field_type, false);

        let must_init = if let FieldDefault::MustInit = field.attr.default_behavior {
            quote!(Def = irisia::element::props::PropInitialized<#field_type>,)
        } else {
            quote!()
        };

        quote! {
            #resolver: irisia::element::props::HelpUpdate<#field_type, #generic_type, #must_init>,
        }
        .to_tokens(&mut output);
    }
    output
}

fn generate_update_with(helper: &GenHelper) -> TokenStream {
    let GenHelper {
        updater_name,
        updater_generics,
        update_result_name: update_result_struct,
        fields,
        ..
    } = helper;

    let iter = fields.iter().map(|HandledField { ident, ty, attr }| {
        let resolver = get_resolver(&attr.value_resolver, &ty, true);
        let new_ident = format_ident!("{ident}_changed");
        quote! {
            #new_ident: !irisia::element::props::HelpUpdate::update(
                &#resolver,
                &mut self.#ident,
                __irisia_updater.#ident,
                true
            ),
        }
    });

    quote! {
        fn update_with(
            &mut self,
            __irisia_updater: #updater_name #updater_generics,
        ) -> #update_result_struct {
            #update_result_struct {
                #(#iter)*
            }
        }
    }
}

fn generate_create_with(helper: &GenHelper) -> TokenStream {
    let GenHelper {
        updater_name,
        updater_generics,
        fields,
        ..
    } = helper;

    let iter = fields.iter().map(|HandledField { ident, ty, attr }| {
        let resolver = get_resolver(&attr.value_resolver, &ty, true);

        let maybe_created = quote! {
            irisia::element::props::HelpCreate::create(
                &#resolver,
                __irisia_updater.#ident
            )
        };

        fn use_defaulter(
            maybe_created: TokenStream,
            default_value: impl ToTokens,
            ret_type: &Type,
        ) -> TokenStream {
            quote! {
                irisia::element::props::Defaulter::with_defaulter(
                    #maybe_created,
                    (|| -> #ret_type { #default_value } as fn() -> #ret_type)
                )
            }
        }

        let final_expr = match &attr.default_behavior {
            FieldDefault::Default => use_defaulter(
                maybe_created,
                quote! {
                    ::std::default::Default::default()
                },
                ty,
            ),
            FieldDefault::DefaultWith(def) => use_defaulter(maybe_created, def, ty),
            FieldDefault::MustInit => quote!(#maybe_created.must_be_initialized()),
        };

        quote! {
            #ident: #final_expr,
        }
    });

    quote! {
        fn create_with(
            __irisia_updater: #updater_name #updater_generics
        ) -> Self {
            Self {
                #(#iter)*
            }
        }
    }
}

fn make_update_reuslt(helper: &GenHelper) -> TokenStream {
    let GenHelper {
        vis,
        update_result_name: update_result,
        fields,
        ..
    } = helper;

    let field_iter = fields.iter().map(|f| format_ident!("{}_changed", f.ident));

    quote! {
        #vis struct #update_result {
            #(pub #field_iter: bool,)*
        }
    }
}
