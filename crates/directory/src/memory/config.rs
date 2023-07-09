use std::sync::Arc;

use utils::config::{utils::AsKey, Config};

use crate::{config::ConfigDirectory, Directory, DirectoryOptions, Principal, Type};

use super::{EmailType, MemoryDirectory};

impl MemoryDirectory {
    pub fn from_config(
        config: &Config,
        prefix: impl AsKey,
    ) -> utils::config::Result<Arc<dyn Directory>> {
        let prefix = prefix.as_key();
        let mut directory = MemoryDirectory {
            opt: DirectoryOptions::from_config(config, prefix.clone())?,
            ..Default::default()
        };

        for lookup_id in config.sub_keys((prefix.as_str(), "users")) {
            let name = config
                .value_require((prefix.as_str(), "users", lookup_id, "name"))?
                .to_string();
            let mut typ = Type::Individual;
            let mut member_of = Vec::new();

            for (_, group) in config.values((prefix.as_str(), "users", lookup_id, "member-of")) {
                if !group.eq_ignore_ascii_case(&directory.opt.superuser_group) {
                    member_of.push(group.to_string());
                } else {
                    typ = Type::Superuser;
                }
            }

            directory.principals.insert(
                name.clone(),
                Principal {
                    name: name.clone(),
                    secrets: config
                        .values((prefix.as_str(), "users", lookup_id, "secret"))
                        .map(|(_, v)| v.to_string())
                        .collect(),
                    typ,
                    description: config
                        .value((prefix.as_str(), "users", lookup_id, "description"))
                        .map(|v| v.to_string()),
                    quota: config
                        .property((prefix.as_str(), "users", lookup_id, "quota"))?
                        .unwrap_or(0),
                    member_of,
                },
            );
            let mut emails = Vec::new();
            for (pos, (_, email)) in config
                .values((prefix.as_str(), "users", lookup_id, "email"))
                .enumerate()
            {
                directory
                    .emails_to_names
                    .entry(email.to_string())
                    .or_default()
                    .push(if pos > 0 {
                        EmailType::Alias(name.clone())
                    } else {
                        EmailType::Primary(name.clone())
                    });

                if let Some((_, domain)) = email.rsplit_once('@') {
                    directory.domains.insert(domain.to_lowercase());
                }

                emails.push(if pos > 0 {
                    EmailType::Alias(email.to_lowercase())
                } else {
                    EmailType::Primary(email.to_lowercase())
                });
            }
            for (_, email) in config.values((prefix.as_str(), "users", lookup_id, "email-list")) {
                directory
                    .emails_to_names
                    .entry(email.to_lowercase())
                    .or_default()
                    .push(EmailType::List(name.clone()));
                if let Some((_, domain)) = email.rsplit_once('@') {
                    directory.domains.insert(domain.to_lowercase());
                }
                emails.push(EmailType::List(email.to_lowercase()));
            }
            directory.names_to_email.insert(name, emails);
        }

        for lookup_id in config.sub_keys((prefix.as_str(), "groups")) {
            let name = config
                .value_require((prefix.as_str(), "groups", lookup_id, "name"))?
                .to_string();
            directory.principals.insert(
                name.clone(),
                Principal {
                    name,
                    secrets: vec![],
                    typ: Type::Group,
                    description: config
                        .value((prefix.as_str(), "groups", lookup_id, "description"))
                        .map(|v| v.to_string()),
                    quota: config
                        .property((prefix.as_str(), "groups", lookup_id, "quota"))?
                        .unwrap_or(0),
                    member_of: config
                        .values((prefix.as_str(), "groups", lookup_id, "member-of"))
                        .map(|(_, v)| v.to_string())
                        .collect(),
                },
            );
        }

        directory
            .domains
            .extend(config.parse_lookup_list((&prefix, "lookup.domains"))?);

        Ok(Arc::new(directory))
    }
}
