use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::ValidateParserError;
use crate::{config::Creator, validate::parser};

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct Metadata {
    pub name: String,
    pub symbol: Option<String>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seller_fee_basis_points: Option<u16>,
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animation_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_url: Option<String>,
    pub attributes: Vec<Attribute>,
    pub properties: Property,
}

impl Metadata {
    pub fn validate(&self, config_data_creators: &Vec<Creator>) -> Result<(), ValidateParserError> {
        parser::check_name(&self.name)?;
        parser::check_url(&self.image)?;

        // If users are using the old format, we do validation on those values.
        if let Some(sfbp) = &self.seller_fee_basis_points {
            parser::check_seller_fee_basis_points(*sfbp)?;
        }
        if let Some(symbol) = &self.symbol {
            parser::check_symbol(symbol)?;
        }

        match &self.properties.creators {
            Some(creators) => {
                parser::check_creators_shares(creators)?;
                parser::validate_metadata_creators(config_data_creators, creators)?;
            }
            None => return Err(ValidateParserError::MissingCreators),
        }

        if let Some(animation_url) = &self.animation_url {
            parser::check_url(animation_url)?;
        }

        if let Some(external_url) = &self.external_url {
            parser::check_url(external_url)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct Property {
    pub files: Vec<FileAttr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creators: Option<Vec<Creator>>,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct Attribute {
    pub trait_type: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct FileAttr {
    pub uri: String,
    #[serde(rename = "type")]
    pub file_type: String,
}
