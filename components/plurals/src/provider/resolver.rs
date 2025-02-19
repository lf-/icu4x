// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

//! Code for resolving plural data.

use super::PluralRuleStringsV1Marker;
use crate::{PluralRuleType, PluralRulesError};
use icu_locid::LanguageIdentifier;
use icu_provider::prelude::*;

#[allow(missing_docs)] // TODO(#1029) - Add missing docs.
pub fn resolve_plural_data<'data, D: DataProvider<'data, PluralRuleStringsV1Marker> + ?Sized>(
    langid: LanguageIdentifier,
    data_provider: &D,
    type_: PluralRuleType,
) -> Result<DataPayload<'data, PluralRuleStringsV1Marker>, PluralRulesError> {
    let key = match type_ {
        PluralRuleType::Cardinal => super::key::CARDINAL_V1,
        PluralRuleType::Ordinal => super::key::ORDINAL_V1,
    };
    Ok(data_provider
        .load_payload(&DataRequest {
            resource_path: ResourcePath {
                key,
                options: ResourceOptions {
                    variant: None,
                    langid: Some(langid),
                },
            },
        })?
        .take_payload()?)
}
