use anyhow::Context;
use fallible_iterator::FallibleIterator;
use inflector::Inflector;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{read_dir, File};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize)]
struct Type {
    name: String,
    shortname: String,
}

#[derive(serde::Deserialize)]
struct Trait {
    name: String,
    shortname: String,
    attributes: Option<Ref>,
    states: Option<Ref>,
    #[serde(default)]
    commands: HashMap<String, Command>,
}

#[derive(serde::Deserialize)]
struct Ref {
    #[serde(rename = "$ref")]
    name: PathBuf,
}

#[derive(serde::Deserialize)]
struct Command {
    shortname: String,
    params: Ref,
}

use json_schema::{
    Definitions, Items, JSONSchema, JSONSchemaObject, SimpleTypes, Type as JsonType,
};

pub struct TypeGenerator<'a> {
    name_map: &'a HashMap<&'a str, &'a str>,
    ignore: &'a HashSet<&'a str>,
    nested_types: HashMap<String, proc_macro2::TokenStream>,
}
impl<'a> TypeGenerator<'a> {
    fn remap_name(&self, gen_name: &str) -> (String, bool) {
        if let Some(name) = self.name_map.get(gen_name) {
            (name.to_string(), true)
        } else {
            (gen_name.replace(".", "_"), false)
        }
    }

    pub fn generate_untagged_enum<'b, I: Iterator<Item = &'b JSONSchema>>(
        &mut self,
        name: &str,
        variants: I,
        definitions: Option<&Definitions>,
    ) -> anyhow::Result<proc_macro2::TokenStream> {
        let ident = quote::format_ident!("{}", name);
        let variants = variants.enumerate().map(|(idx, variant)| {
            if let JSONSchema::JSONSchemaObject(schema) = variant {
                let (variant_name, _) = self.remap_name(&format!("{}_{}", name, idx));
                let ident = quote::format_ident!("{}", variant_name);
                if let Ok(struct_body) =
                    self.generate_type_struct(&variant_name, schema, false, definitions)
                {
                    quote::quote! {
                        #ident {
                            #(#struct_body,)*
                        }
                    }
                } else {
                    let type_name = self
                        .parse_type_from_json_schema(&variant_name, schema, definitions)
                        .unwrap()
                        .unwrap();
                    quote::quote! {
                        #ident(#type_name)
                    }
                }
            } else {
                panic!("Not avaiable");
            }
        });

        Ok(quote::quote! {
            #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
            #[serde(untagged)]
            pub enum #ident {
                #(#variants,)*
            }
        })
    }

    pub fn generate_tagged_enum<'b, I: Iterator<Item = &'b JSONSchema> + Clone>(
        &mut self,
        name: &str,
        variants: I,
        definitions: Option<&Definitions>,
    ) -> anyhow::Result<proc_macro2::TokenStream> {
        let sample = variants.clone().next().unwrap();
        let schema = if let JSONSchema::JSONSchemaObject(schema) = sample {
            schema
        } else {
            panic!("Not available");
        };

        if let Some(_enum) = &schema._enum {
            return Ok(self.generate_simple_enum(
                name,
                variants.clone().flat_map(|o| {
                    if let JSONSchema::JSONSchemaObject(schema) = &o {
                        schema._enum.as_ref().unwrap().iter()
                    } else {
                        panic!("Invalid type");
                    }
                }),
            ));
        }

        let props = if let Some(props) = &schema.properties {
            props
        } else {
            return Err(anyhow::anyhow!("not a tagged enum"));
        };

        let keys = props.keys().collect::<HashSet<_>>();
        let (single_key, common_keys): (bool, HashSet<&String>) =
            variants
                .clone()
                .fold((keys.len() == 1, keys), |(single_key, k), o| {
                    if let JSONSchema::JSONSchemaObject(s) = o {
                        let keys = s
                            .properties
                            .as_ref()
                            .unwrap()
                            .keys()
                            .map(|o| &*o)
                            .collect::<HashSet<_>>();
                        (
                            single_key && (keys.len() == 1),
                            k.intersection(&keys).map(|o| &**o).collect(),
                        )
                    } else {
                        panic!("Invalid type");
                    }
                });
        let tag_key = common_keys
            .into_iter()
            .find(|k| {
                if let JSONSchema::JSONSchemaObject(s) = props.get(k.as_str()).unwrap() {
                    if let Some(e) = &s._enum {
                        e.len() == 1
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .ok_or_else(|| anyhow::anyhow!("Not a tagged enum"))?;

        if single_key {
            Ok(self.generate_simple_enum(
                name,
                variants.map(|o| {
                    if let JSONSchema::JSONSchemaObject(schema) = &o {
                        if let JSONSchema::JSONSchemaObject(schema) =
                            schema.properties.as_ref().unwrap().get(tag_key).unwrap()
                        {
                            return schema._enum.as_ref().unwrap().get(0).unwrap();
                        }
                    }

                    panic!("Invalid type");
                }),
            ))
        } else {
            let variants = variants.flat_map(|o| {
                if let JSONSchema::JSONSchemaObject(schema) = &o {
                    let properties = schema.properties.as_ref().unwrap();
                    let tag_names = if let JSONSchema::JSONSchemaObject(schema) =
                        properties.get(tag_key).unwrap()
                    {
                        schema
                            ._enum
                            .as_ref()
                            .unwrap()
                            .iter()
                            .map(|name| {
                                if let serde_json::Value::String(name) = name {
                                    (name, quote::format_ident!("{}", &name.to_pascal_case()))
                                } else {
                                    panic!("Invalid type");
                                }
                            })
                            .collect::<Vec<_>>()
                    } else {
                        panic!("Invalid type");
                    };

                    if properties.len() == 1 {
                        tag_names
                            .into_iter()
                            .map(|(tag_name, tag_ident)| {
                                quote::quote! {
                                    #[serde(rename = #tag_name)]
                                    #tag_ident
                                }
                            })
                            .collect::<Vec<_>>()
                    } else {
                        let representetive_tag_name = tag_names.get(0).unwrap().0;
                        let fields = properties
                            .iter()
                            .filter_map(|(k, v)| {
                                if k == tag_key {
                                    return None;
                                }

                                let field_name = k.to_pascal_case();
                                let (field_type_name, has_remap) = self.remap_name(&format!(
                                    "{}_{}_{}",
                                    name, representetive_tag_name, field_name
                                ));
                                if !has_remap {
                                    eprintln!("warning={} has no remap", field_type_name);
                                }
                                let field_ident = quote::format_ident!("{}", k.to_snake_case());
                                let field_type = if let JSONSchema::JSONSchemaObject(schema) = &v {
                                    self.parse_type_from_json_schema(
                                        &field_type_name,
                                        schema,
                                        definitions,
                                    )
                                    .unwrap()
                                    .unwrap()
                                } else {
                                    panic!("Invalid type");
                                };
                                Some(quote::quote! {
                                    #[serde(rename = #k)]
                                    #field_ident: #field_type
                                })
                            })
                            .collect::<Vec<_>>();

                        tag_names
                            .into_iter()
                            .map(move |(tag_name, tag_ident)| {
                                quote::quote! {
                                    #[serde(rename = #tag_name)]
                                    #tag_ident{#(#fields,)*}
                                }
                            })
                            .collect::<Vec<_>>()
                    }
                } else {
                    panic!("Invalid type");
                }
            });
            let ident = quote::format_ident!("{}", name);
            let decl = quote::quote! {
                #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
                #[serde(tag = #tag_key)]
                pub enum #ident {
                    #(#variants,)*
                }
            };
            self.nested_types.insert(name.to_string(), decl);
            Ok(quote::quote!(#ident))
        }
    }

    fn generate_type_enum(
        &mut self,
        name: &str,
        variants: &Vec<JSONSchema>,
        definitions: Option<&Definitions>,
    ) -> anyhow::Result<proc_macro2::TokenStream> {
        let ident = quote::format_ident!("{}", &name);

        let variants =
            fallible_iterator::convert(variants.iter().enumerate().filter_map(|(idx, variant)| {
                let gen_name = format!("{}_{}", name, idx);
                if let JSONSchema::JSONSchemaObject(prop) = variant {
                    let (inner_name, has_remap) = self.remap_name(&gen_name);
                    let variant_name = if has_remap {
                        quote::format_ident!("{}", &inner_name)
                    } else {
                        quote::format_ident!("Variant{}", idx)
                    };

                    if let Ok(variants) =
                        self.generate_type_struct(&inner_name, prop, false, definitions)
                    {
                        if !has_remap {
                            eprintln!("cargo:warning={} has no mapped name", &inner_name)
                        }
                        Some(Ok(quote::quote! {
                            #variant_name {
                                #(#variants,)*
                            }
                        }))
                    } else {
                        let type_name =
                            match self.parse_type_from_json_schema(&inner_name, &prop, definitions)
                            {
                                Ok(name) => name,
                                Err(e) => return Some(Err(e)),
                            };

                        Some(Ok(quote::quote! {
                            #variant_name(#type_name)
                        }))
                    }
                } else {
                    None
                }
            }))
            .collect::<Vec<_>>()?;

        Ok(quote::quote! {
            #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
            #[serde(untagged)]
            pub enum #ident {
                #(#variants,)*
            }
        })
    }

    fn generate_type_struct(
        &mut self,
        name: &str,
        schema: &JSONSchemaObject,
        nested: bool,
        definitions: Option<&Definitions>,
    ) -> anyhow::Result<Vec<proc_macro2::TokenStream>> {
        let required: HashSet<String> = schema
            .required
            .as_ref()
            .map(|r| r.iter().cloned().collect())
            .unwrap_or_default();
        let mut props = schema
            .properties
            .as_ref()
            .map(|properties| -> anyhow::Result<_> {
                fallible_iterator::convert(properties.iter().filter_map(
                    |(prop_name, prop_detail)| -> Option<anyhow::Result<_>> {
                        if let JSONSchema::JSONSchemaObject(prop) = prop_detail {
                            let type_name = match self.parse_type_from_json_schema(
                                &format!("{}_{}", &name, &prop_name),
                                &prop,
                                definitions,
                            ) {
                                Ok(name) => name,
                                Err(e) => return Some(Err(e)),
                            };
                            if let Some(type_name) = type_name {
                                let (attr, type_name) = if required.contains(prop_name)
                                    || type_name.to_string().starts_with("Vec<")
                                {
                                    (quote::quote!(), type_name)
                                } else {
                                    (quote::quote!(#[serde(skip_serializing_if = "Option::is_none")]), quote::quote! { Option<#type_name> })
                                };
                                let comment = if let Some(description) = &prop.description {
                                    quote::quote! { #[doc = #description ] }
                                } else {
                                    quote::quote! {}
                                };
                                let name = quote::format_ident!("{}", prop_name.to_snake_case());

                                let vis = if nested {
                                    quote::quote! { pub }
                                } else {
                                    quote::quote!()
                                };

                                Some(Ok((
                                    prop_name.clone(),
                                    quote::quote! {
                                        #comment
                                        #attr
                                        #[serde(rename = #prop_name)]
                                        #vis #name: #type_name
                                    },
                                )))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    },
                ))
                .collect::<Vec<_>>()
            })
            .transpose()?
            .unwrap_or_default();
        props.sort_by(|a, b| std::cmp::Ord::cmp(&a.0, &b.0));
        let mut props = props.into_iter().map(|(_, t)| t).collect::<Vec<_>>();
        let additional_properties = schema
            .additional_properties
            .as_ref()
            .and_then(|obj| match obj.as_ref() {
                JSONSchema::JSONSchemaObject(nested_type) => {
                    let name = format!("{}_ADDITIONAL_PROPS", name);
                    let nested_type_name = self
                        .parse_type_from_json_schema(&name, nested_type, definitions)
                        .unwrap();
                    Some(quote::quote! {
                        #[serde(flatten)]
                        additional_values: std::collections::HashMap<String, #nested_type_name>
                    })
                }
                JSONSchema::JSONSchemaBoolean(true) => Some(quote::quote! {
                    #[serde(flatten)]
                    additional_values: std::collections::HashMap<String, serde_json::Value>
                }),
                JSONSchema::JSONSchemaBoolean(false) => None,
            });

        if let Some(additional_properties) = additional_properties {
            props.push(additional_properties);
        }

        if let Some(one_of) = schema.one_of.as_ref() {
            let (name, has_remap) = self.remap_name(&format!("{}_DETAILS", name));
            let untagged_enum = self.generate_untagged_enum(&name, one_of.iter(), definitions)?;
            self.insert_nested_type(&name, has_remap, untagged_enum);
            let ident = quote::format_ident!("_details");
            let type_ident = quote::format_ident!("{}", name);
            props.push(quote::quote! {
                #[serde(flatten)]
                #ident: #type_ident
            });
        }

        Ok(props)
    }

    fn generate_simple_enum<'b, I: Iterator<Item = &'b serde_json::Value> + 'b>(
        &mut self,
        name: &str,
        enum_items: I,
    ) -> proc_macro2::TokenStream {
        let variants = enum_items.map(|variant| {
            if let serde_json::Value::String(val) = variant {
                let ident = quote::format_ident!("{}", &val.to_pascal_case().replace(".", "_"));
                quote::quote! {
                    #[serde(rename = #val)]
                    #ident
                }
            } else {
                panic!("not supported type of enum");
            }
        });

        let ident = quote::format_ident!("{}", name);
        self.nested_types.insert(
            name.to_string(),
            quote::quote! {
                #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
                pub enum #ident {
                    #(#variants,)*
                }
            },
        );

        quote::quote!(#ident)
    }

    fn parse_type_from_json_schema(
        &mut self,
        name: &str,
        prop: &JSONSchemaObject,
        definitions: Option<&Definitions>,
    ) -> anyhow::Result<Option<proc_macro2::TokenStream>> {
        if let Some(_ref) = &prop._ref {
            let name = _ref.rsplit_once("/").unwrap().1;
            let def = definitions.unwrap().get(name).unwrap();
            if let JSONSchema::JSONSchemaObject(prop) = &def {
                return self.parse_type_from_json_schema(&name.to_pascal_case(), prop, definitions);
            } else {
                return Ok(None);
            }
        }
        if let Some(_enum) = &prop._enum {
            return Ok(Some(self.generate_simple_enum(name, _enum.iter())));
        }
        if let Some(one_of) = &prop.one_of {
            let filtered_one_of: Vec<_> = one_of
                .iter()
                .enumerate()
                .filter_map(|(idx, schema)| {
                    let variant_name = format!("{}_{}", name, idx);
                    if self.ignore.contains(variant_name.as_str()) {
                        None
                    } else {
                        Some(schema)
                    }
                })
                .collect();
            let (name, has_remap) = self.remap_name(name);
            if !has_remap {
                eprintln!("{} has no remap", name);
            }
            if filtered_one_of.len() == 1 {
                if let JSONSchema::JSONSchemaObject(prop) = filtered_one_of.get(0).unwrap() {
                    return self.parse_type_from_json_schema(&name, prop, definitions);
                } else {
                    unreachable!();
                }
            } else if let Ok(tagged) =
                self.generate_tagged_enum(&name, filtered_one_of.into_iter(), definitions)
            {
                return Ok(Some(tagged));
            }
        }

        if let Some(Items::JSONSchema(items)) = &prop.items {
            let (name, has_remap) = self.remap_name(&name);

            if !has_remap {
                eprintln!("cargo:warning={} has no remap", &name);
            }

            if let JSONSchema::JSONSchemaObject(items) = items.as_ref() {
                let inner_type = self
                    .parse_type_from_json_schema(&name, items, definitions)?
                    .unwrap();
                return Ok(Some(quote::quote!(Vec<#inner_type>)));
            }
        }
        let type_name = match &prop._type {
            Some(JsonType::SimpleTypes(SimpleTypes::Boolean)) => Some(quote::quote!(bool)),
            Some(JsonType::SimpleTypes(SimpleTypes::String)) => Some(quote::quote!(String)),
            Some(JsonType::SimpleTypes(SimpleTypes::Number)) => Some(quote::quote!(f64)),
            Some(JsonType::SimpleTypes(SimpleTypes::Integer)) => {
                let mut is_unsigned = false;
                let mut minimal_width = 0;
                if let Some(minimum) = prop.minimum {
                    if minimum >= 0.0 {
                        is_unsigned = true;
                    } else {
                        if minimum >= i8::MIN as f64 {
                            minimal_width = 1;
                        } else if minimum >= i16::MIN as f64 {
                            minimal_width = 2;
                        } else if minimum >= i32::MIN as f64 {
                            minimal_width = 4;
                        } else if minimum >= i64::MIN as f64 {
                            minimal_width = 8;
                        } else {
                            unreachable!("Too small minimum");
                        }
                    }
                }
                if let Some(maximum) = prop.maximum {
                    if is_unsigned {
                        Some(if maximum <= u8::MAX as f64 {
                            quote::quote!(u8)
                        } else if maximum <= u16::MAX as f64 {
                            quote::quote!(u16)
                        } else if maximum <= u32::MAX as f64 {
                            quote::quote!(u32)
                        } else if maximum <= u64::MAX as f64 {
                            quote::quote!(u64)
                        } else {
                            unreachable!("Too large maximum")
                        })
                    } else {
                        let width = std::cmp::max(
                            minimal_width,
                            if maximum <= i8::MAX as f64 {
                                1
                            } else if maximum <= i16::MAX as f64 {
                                2
                            } else if maximum <= i32::MAX as f64 {
                                4
                            } else if maximum <= i64::MAX as f64 {
                                8
                            } else {
                                unreachable!("Too large maximum")
                            },
                        );
                        Some(match width {
                            1 => quote::quote!(i8),
                            2 => quote::quote!(i16),
                            4 => quote::quote!(i32),
                            8 => quote::quote!(i64),
                            _ => unreachable!(),
                        })
                    }
                } else {
                    Some(quote::quote!(i64))
                }
            }
            Some(JsonType::SimpleTypes(SimpleTypes::Object)) => {
                let (name, has_remap) = self.remap_name(name);

                let inner_type = if let Some(one_of) = &prop.one_of {
                    let inner_type = self.generate_type_enum(&name, &one_of, definitions)?;
                    inner_type
                } else {
                    let inner_type = self.generate_type_struct(&name, &prop, true, definitions)?;
                    let ident = quote::format_ident!("{}", name);
                    quote::quote! {
                        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
                        pub struct #ident {
                            #(#inner_type,)*
                        }
                    }
                };

                self.insert_nested_type(&name, has_remap, inner_type);

                let name = quote::format_ident!("{}", name);

                Some(quote::quote!(#name))
            }
            Some(JsonType::SimpleTypes(SimpleTypes::Array)) => {
                unreachable!();
            }
            None if prop.minimum.is_some() || prop.maximum.is_some() => Some(quote::quote!(f64)),
            t => {
                println!("cargo:warning=unknown type - {:?} on {}", t, name);
                None
            }
        };

        Ok(type_name)
    }

    fn insert_nested_type(
        &mut self,
        name: &str,
        has_remap: bool,
        inner_type: proc_macro2::TokenStream,
    ) {
        if let Some(pre_generated_type) = self.nested_types.get(name) {
            let a = pre_generated_type.to_string();
            let b = inner_type.to_string();
            if a != b {
                eprintln!("cargo:error={} is duplicated remap. but they are different.\n=== A ===\n{}\n=== B ===\n{}", &name, &a, &b);
            }
        } else {
            self.nested_types.insert(name.to_string(), inner_type);

            if !has_remap {
                eprintln!("cargo:warning={} has no remap", &name);
            }
        }
    }
}

pub fn generate<PS: AsRef<Path>, PO: AsRef<Path>>(schema_root: PS, out: PO) -> anyhow::Result<()> {
    let name_map: HashMap<&'static str, &'static str> = [
        ("ColorSetting_color", "ColorSetting"),
        ("ColorSetting_0", "ColorSettingKelvin"),
        ("ColorSetting_1", "ColorRgb"),
        ("ColorSetting_2", "ColorSettingHsv"),
        ("ColorSettingHsv_spectrumHsv", "SpectrumHsv"),
        ("ColorAbsolute_color", "ColorAbsolute"),
        ("ColorAbsolute_0", "ColorAbsoluteKelvin"),
        ("ColorAbsolute_1", "ColorRgb"),
        ("ColorAbsolute_2", "ColorAbsoluteHSV"),
        ("ColorAbsoluteHSV_spectrumHSV", "SpectrumHsv"),
        (
            "NetworkControl_lastNetworkUploadSpeedTest",
            "LastNetworkUploadSpeedTest",
        ),
        ("Dispense_dispenseItems", "DispenseItem"),
        ("DispenseItem_amountRemaining", "DispenseRemaining"),
        ("DispenseItem_amountLastDispensed", "DispenseRemaining"),
        (
            "NetworkControl_guestNetworkSettings",
            "GuestNetworkSettings",
        ),
        ("InputSelector_availableInputs", "Input"),
        (
            "NetworkControl_lastNetworkDownloadSpeedTest",
            "LastNetworkDownloadSpeedTest",
        ),
        ("NetworkControl_networkSettings", "NetworkSettings"),
        ("Toggles_currentToggleSettings", "CurrentToggleSettings"),
        ("Modes_currentModeSettings", "CurrentModeSettings"),
        (
            "TemperatureSetting_thermostatTemperatureRange",
            "TemperatureRange",
        ),
        ("TemperatureControl_temperatureRange", "TemperaturRange"),
        ("TemperatureControl_temperatureUnitForUX", "TemperaturUnit"),
        (
            "TemperatureSetting_thermostatTemperatureUnit",
            "TemperaturUnit",
        ),
        (
            "TemperatureSetting_availableThermostatModes",
            "ThermostatMode",
        ),
        ("TemperatureSetting_DETAILS", "TemperatureSettingDetail"),
        ("TemperatureSettingDetail_0", "SingleTemperaturSetting"),
        ("SingleTemperaturSetting_thermostatMode", "ThermostatMode"),
        ("TemperatureSettingDetail_1", "RangeTemperaturSetting"),
        ("RangeTemperaturSetting_thermostatMode", "ThermostatMode"),
        ("TemperatureSetting_activeThermostatMode", "ThermostatMode"),
        ("ThermostatSetMode_thermostatMode", "ThermostatMode"),
        ("Cook_supportedCookingModes", "CookingMode"),
        ("Rotation_rotationDegreesRange", "RotationDegreesRange"),
        ("SensorState_currentSensorStateData", "SensorState"),
        (
            "SensorState_RainDetection_CurrentSensorState",
            "RainDetectionSensorState",
        ),
        ("SensorState_sensorStatesSupported", "SensorStateSupported"),
        (
            "SensorStateSupported_PM2_5_NumericCapabilities",
            "PmSensorStateSupported",
        ),
        ("PmSensorStateSupported_rawValueUnit", "PmSensorUnit"),
        (
            "SensorState_currentSensorStateData_CurrentSensorState",
            "CurrentSensorState",
        ),
        ("SensorState_WaterLeak_CurrentSensorState", "WaterLeakSensorState"),
        ("SensorStateSupported_AirQuality_DescriptiveCapabilities", "AirQualitySensorStateSupported"),
        (
            "EnergyStorage_descriptiveCapacityRemaining",
            "CapacityRemaning",
        ),
        ("EnergyStorage_capacityRemaining", "Capacity"),
        ("EnergyStorage_capacityUntilFull", "Capacity"),
        ("Capacity_unit", "CapacityUnit"),
        ("OpenClose_openDirection", "OpenDirection"),
    ]
    .into_iter()
    .collect();

    let mut out = File::create(out)?;
    let schema_root = schema_root.as_ref();

    // generate Type
    {
        let types = fallible_iterator::convert(read_dir(schema_root.join("types"))?.map(
            |dir| -> anyhow::Result<_> {
                let dir = dir?;
                let mut scheme_path = dir.path();
                scheme_path.push("index.yaml");

                let scheme_file = File::open(scheme_path)?;
                let device_type: Type =
                    serde_yaml::from_reader(scheme_file).with_context(|| {
                        format!("while parsing {}", dir.file_name().to_str().unwrap())
                    })?;
                let comment = &device_type.shortname;
                let type_name = &device_type.name;
                let ident = quote::format_ident!(
                    "{}",
                    device_type
                        .name
                        .rsplit_once('.')
                        .ok_or_else(|| anyhow::anyhow!("Incorrect name"))?
                        .1
                        .to_pascal_case()
                );

                Ok(quote::quote! {
                    #[doc = #comment]
                    #[serde(rename = #type_name)]
                    #ident
                })
            },
        ))
        .collect::<Vec<_>>()?;

        writeln!(
            out,
            "{}",
            quote::quote! {
                #[derive(Debug, Clone, Copy, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
                pub enum Type {
                    #(#types,)*
                }
            }
        )?;
    };

    let mut attributes = BTreeMap::new();
    let mut states = BTreeMap::new();
    let mut commands = BTreeMap::new();

    // generate Trait
    {
        let traits = fallible_iterator::convert(read_dir(schema_root.join("traits"))?.map(
            |dir| -> anyhow::Result<_> {
                let dir = dir?;
                let scheme_dir = dir.path();
                let scheme_path = scheme_dir.join("index.yaml");

                let scheme_file = File::open(scheme_path)?;
                let device_trait: Trait =
                    serde_yaml::from_reader(scheme_file).with_context(|| {
                        format!("while parsing {}", dir.file_name().to_str().unwrap())
                    })?;
                let name = device_trait
                    .name
                    .rsplit_once('.')
                    .ok_or_else(|| anyhow::anyhow!("Incorrect name"))?
                    .1
                    .to_pascal_case();
                let comment = &device_trait.shortname;
                let trait_name = &device_trait.name;
                let ident = quote::format_ident!("{}", &name);

                if let Some(attribute) = device_trait.attributes {
                    attributes.insert(name.clone(), scheme_dir.join(attribute.name));
                }
                if let Some(state) = device_trait.states {
                    states.insert(name.clone(), scheme_dir.join(state.name));
                }
                for (command_name, command) in device_trait.commands.into_iter() {
                    commands.insert(command_name, scheme_dir.join(command.params.name));
                }

                Ok(quote::quote! {
                    #[doc = #comment]
                    #[serde(rename = #trait_name)]
                    #ident
                })
            },
        ))
        .collect::<Vec<_>>()?;

        writeln!(
            out,
            "{}",
            quote::quote! {
                #[derive(Debug, Clone, Copy, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
                pub enum Trait {
                    #(#traits,)*
                }
            }
        )?;
    };

    let mut generator = TypeGenerator {
        name_map: &name_map,
        ignore: &["TemperatureSetting_availableThermostatModes_0"]
            .into_iter()
            .collect(),
        nested_types: Default::default(),
    };

    // generate State
    {
        let state_variants =
            fallible_iterator::convert(states.iter().map(|(name, path)| -> anyhow::Result<_> {
                let state_scheme = File::open(path)?;
                let schema: JSONSchemaObject = serde_json::from_reader(state_scheme)?;

                let struct_content = generator.generate_type_struct(
                    name,
                    &schema,
                    false,
                    schema.definitions.as_ref(),
                )?;
                let ident = quote::format_ident!("{}", name);
                Ok(quote::quote! {
                    #ident{#(#struct_content,)*}
                })
            }))
            .collect::<Vec<_>>()?;

        writeln!(
            out,
            "{}",
            quote::quote! {
                #[derive(Debug, Clone, serde::Serialize)]
                #[serde(rename_all = "snake_case")]
                pub enum State {
                    #(#state_variants,)*
                }

                #[derive(Debug, Clone)]
                #[repr(transparent)]
                pub struct States(
                    pub Vec<State>
                );
            }
        )?;
    };

    // generate Attributes
    {
        let attribute_variants = fallible_iterator::convert(attributes.iter().map(
            |(name, path)| -> anyhow::Result<_> {
                let state_scheme = File::open(path)?;
                let schema: JSONSchemaObject = serde_json::from_reader(state_scheme)?;

                let struct_content = generator.generate_type_struct(
                    name,
                    &schema,
                    false,
                    schema.definitions.as_ref(),
                )?;
                let ident = quote::format_ident!("{}", name);
                Ok(quote::quote! {
                    #ident{#(#struct_content,)*}
                })
            },
        ))
        .collect::<Vec<_>>()?;

        writeln!(
            out,
            "{}",
            quote::quote! {
                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                #[serde(rename_all = "snake_case")]
                pub enum Attribute {
                    #(#attribute_variants,)*
                }

                #[derive(Debug, Clone)]
                #[repr(transparent)]
                pub struct Attributes(
                    pub Vec<Attribute>
                );
            }
        )?;
    };

    // generate Command
    {
        let command_variants =
            fallible_iterator::convert(commands.iter().map(|(full_name, path)| -> anyhow::Result<_> {
                let state_scheme = File::open(path)?;
                let schema: JSONSchemaObject = serde_json::from_reader(state_scheme)?;

                let name = full_name.rsplit_once(".").unwrap().1.to_pascal_case();
                let struct_content = generator.generate_type_struct(
                    &name,
                    &schema,
                    false,
                    schema.definitions.as_ref(),
                )?;
                let ident = quote::format_ident!("{}", name);
                Ok(quote::quote! {
                    #[serde(rename = #full_name)]
                    #ident{#(#struct_content,)*}
                })
            }))
            .collect::<Vec<_>>()?;

        writeln!(
            out,
            "{}",
            quote::quote! {
                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                #[serde(tag = "command", content = "params")]
                pub enum Command {
                    #(#command_variants,)*
                }
            }
        )?;
    };

    //generate errors
    {
        let scheme_path = schema_root.join("platform/errors.schema.json");

        let scheme_file = File::open(scheme_path)?;
        let schema: JSONSchemaObject = serde_json::from_reader(scheme_file)?;
        let variants = schema._enum.unwrap();
        let variants = variants
            .iter()
            .map(|variant| quote::format_ident!("{}", variant.as_str().unwrap().to_pascal_case()));

        writeln!(
            out,
            "{}",
            quote::quote! {
                #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                #[serde(rename_all = "camelCase")]
                pub enum Error {
                    #(#variants,)*
                }
            }
        )?;
    };

    for nested_type in generator.nested_types.values() {
        writeln!(out, "{}", nested_type)?;
    }

    Ok(())
}
