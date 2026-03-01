/*
 * Copyright (c) Radical HQ Limited
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use crate::{error::Result, output::output};

#[derive(Debug, clap::Parser)]
pub struct SquashOptions {
    /// Source revision to squash (default: @)
    #[clap(short = 'r', long)]
    pub revision: Option<String>,

    /// Target revision to squash into (default: parent of source)
    #[clap(long)]
    pub into: Option<String>,

    /// Do not copy the pushed git commit hash to the clipboard
    #[clap(long)]
    pub no_clipboard: bool,
}

pub async fn squash(
    opts: SquashOptions,
    jj: &crate::jj::Jujutsu,
    gh: &mut crate::github::GitHub,
    config: &crate::config::Config,
) -> Result<()> {
    // Resolve source revision (default @, matching jj squash behavior)
    let source_revision = opts.revision.as_deref().unwrap_or("@");

    // Get the source commit to read its description
    let source_commit = jj.get_prepared_commit_for_revision(config, source_revision)?;
    let source_description = jj.get_revision_description(source_revision)?;

    // Get the source title for output
    let source_title = source_commit
        .message
        .get(&crate::message::MessageSection::Title)
        .cloned()
        .unwrap_or_else(|| "(no title)".to_string());

    output("⏸️ ", &format!("Squashing: {}", source_title))?;

    // Squash the source into target, preserving the target's commit message
    jj.squash_revision(source_revision, opts.into.as_deref())?;

    // Resolve target to OID after squash.
    // When no --into is provided, jj squash puts changes into the parent,
    // so @- is always the target and avoids divergent change ID issues.
    let target_ref = opts.into.as_deref().unwrap_or("@-");
    let target_oid = jj.resolve_revision_to_commit_id(target_ref)?;

    // Create DiffOptions to delegate to diff command
    let diff_opts = crate::commands::diff::DiffOptions {
        all: false,
        update_message: false,
        draft: false,
        message: Some(source_description),
        no_clipboard: opts.no_clipboard,
        cherry_pick: false,
        base: None,
        revision: Some(target_oid.to_string()),
        dry_run: false,
    };

    // Delegate to diff command
    crate::commands::diff::diff(diff_opts, jj, gh, config).await
}
