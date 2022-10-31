// Copyright 2018-2022 Parity Technologies (UK) Ltd.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::GenerateCode;

use derive_more::From;
use proc_macro2::TokenStream as TokenStream2;
use quote::{
    quote,
    quote_spanned,
};

/// Generates code for an event definition.
#[derive(From)]
pub struct EventDefinition<'a> {
    event_def: &'a ir::InkEventDefinition,
}

impl GenerateCode for EventDefinition<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let event_enum = self.generate_event_enum();
        let event_info_impls = self.generate_event_info_impl();
        let event_variant_info_impls = self.generate_event_variant_info_impls();
        let event_metadata_impl = self.generate_event_metadata_impl();
        let topics_impl = self.generate_topics_impl();
        let topics_guard = self.generate_topics_guard();
        quote! {
            #event_enum
            #event_info_impls
            #event_variant_info_impls
            #event_metadata_impl
            #topics_impl
            #topics_guard
        }
    }
}

impl<'a> EventDefinition<'a> {
    fn generate_event_enum(&'a self) -> TokenStream2 {
        let span = self.event_def.span();
        let event_enum = &self.event_def.item;
        quote_spanned!(span =>
            #[derive(::scale::Encode, ::scale::Decode)]
            #event_enum
        )
    }

    fn generate_event_info_impl(&self) -> TokenStream2 {
        let span = self.event_def.span();
        let event_ident = self.event_def.ident();

        quote_spanned!(span=>
            impl ::ink::reflect::EventInfo for #event_ident {
                const PATH: &'static str = ::core::concat!(
                    ::core::module_path!(),
                    "::",
                    ::core::stringify!(#event_ident)
                );
            }
        )
    }

    fn generate_event_variant_info_impls(&self) -> TokenStream2 {
        let span = self.event_def.span();
        let event_ident = self.event_def.ident();

        let impls = self.event_def.variants().map(|ev| {
            let event_variant_ident = ev.ident();
            let index = ev.index();
            quote_spanned!(span=>
                impl ::ink::reflect::EventVariantInfo<#index> for #event_ident {
                    const NAME: &'static str = ::core::stringify!(#event_variant_ident);
                    const SIGNATURE_TOPIC: [u8; 32] = ::ink::primitives::event_signature_topic(
                        <Self as ::ink::reflect::EventInfo>::PATH,
                        <Self as ::ink::reflect::EventVariantInfo<#index>>::NAME,
                    );
                }
            )
        });
        quote_spanned!(span=>
            #(
                #impls
            )*
        )
    }

    fn generate_event_metadata_impl(&self) -> TokenStream2 {
        let event_metadata = super::metadata::EventMetadata::from(self.event_def);
        event_metadata.generate_code()
    }

    /// Generate checks to guard against too many topics in event definitions.
    fn generate_topics_guard(&self) -> TokenStream2 {
        let span = self.event_def.span();
        let event_ident = self.event_def.ident();
        // todo: [AJ] check if event signature topic should be included here (it is now, wasn't before)
        let len_topics = self.event_def.max_len_topics();

        quote_spanned!(span=>
            impl ::ink::codegen::EventLenTopics for #event_ident {
                type LenTopics = ::ink::codegen::EventTopics<#len_topics>;
            }
        )
    }

    fn generate_topics_impl(&self) -> TokenStream2 {
        let span = self.event_def.span();
        let event_ident = self.event_def.ident();

        let variant_match_arms = self
            .event_def
            .variants()
            .map(|variant| {
                let span = variant.span();
                let variant_ident = variant.ident();
                let (field_bindings, field_topics): (Vec<_>, Vec<_>) = variant.fields()
                    .filter(|field| field.is_topic)
                    .map(|field| {
                        let field_type = field.ty();
                        let field_ident = field.ident();
                        let push_topic =
                            quote_spanned!(span =>
                                .push_topic::<::ink::env::topics::PrefixedValue<#field_type>>(
                                    &::ink::env::topics::PrefixedValue {
                                        // todo: figure out whether we even need to include a prefix here?
                                        // Previously the prefix would be the full field path e.g.
                                        // erc20::Event::Transfer::from + value.
                                        // However the value on its own might be sufficient, albeit
                                        // requiring combination with the signature topic and some
                                        // metadata to determine whether a topic value belongs to a
                                        // specific field of a given Event variant. The upside is that
                                        // indexers can use the unhashed value for meaningful topics
                                        // e.g. addresses < 32 bytes. If the prefix is included we
                                        // will always require to hash the value so need any indexer
                                        // would not be able to go from hash > address.
                                        prefix: &[],
                                        value: #field_ident,
                                    }
                                )
                            );
                        let binding = quote_spanned!(span=> ref #field_ident);
                        (binding, push_topic)
                    })
                    .unzip();

                let index = variant.index();
                let event_signature_topic = match variant.anonymous() {
                    true => None,
                    false => {
                        Some(quote_spanned!(span=>
                            .push_topic::<::ink::env::topics::PrefixedValue<()>>(
                                &::ink::env::topics::PrefixedValue {
                                    prefix: &<#event_ident as ::ink::reflect::EventVariantInfo<#index>>::SIGNATURE_TOPIC,
                                    value: &(),
                                }
                            )
                        ))
                    }
                };

                let remaining_topics_ty = match variant.len_topics() {
                    0 => quote_spanned!(span=> ::ink::env::topics::state::NoRemainingTopics),
                    n => {
                        quote_spanned!(span=> [::ink::env::topics::state::HasRemainingTopics; #n])
                    }
                };

                quote_spanned!(span=>
                    Self::#variant_ident { #( #field_bindings, )* .. } => {
                        builder
                            .build::<#remaining_topics_ty>()
                            #event_signature_topic
                            #(
                                #field_topics
                            )*
                            .finish()
                    }
                )
            });

        quote_spanned!(span =>
            const _: () = {
                impl ::ink::env::Topics for #event_ident {
                    fn topics<E, B>(
                        &self,
                        builder: ::ink::env::topics::TopicsBuilder<::ink::env::topics::state::Uninit, E, B>,
                    ) -> <B as ::ink::env::topics::TopicsBuilderBackend<E>>::Output
                    where
                        E: ::ink::env::Environment,
                        B: ::ink::env::topics::TopicsBuilderBackend<E>,
                    {
                        match self {
                            #(
                                #variant_match_arms
                            )*
                        }
                    }
                }
            };
        )
    }
}
