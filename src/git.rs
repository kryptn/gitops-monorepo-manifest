use git2::{Commit, DiffOptions, Error, Repository, Tree};

pub fn get_branch_commit<'a>(
    repo: &'a Repository,
    branch_name: &str,
) -> Result<Commit<'a>, git2::Error> {
    let reference = repo.find_branch(branch_name, git2::BranchType::Local)?;
    let commit = reference.into_reference().peel_to_commit()?;
    Ok(commit)
}

pub fn get_branch_commit_hash(repo: &Repository, branch_name: &str) -> Result<String, git2::Error> {
    let reference = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .or(repo.find_branch(branch_name, git2::BranchType::Remote))?;
    let commit = reference.into_reference().peel_to_commit()?;
    Ok(commit.id().to_string())
}

pub fn get_merge_base(repo: &Repository, base: &str, head: &str) -> Result<String, git2::Error> {
    let base_commit = get_branch_commit(repo, base)?;
    let head_commit = get_branch_commit(repo, head)?;

    let merge_base = repo.merge_base(base_commit.id(), head_commit.id())?;
    Ok(merge_base.to_string())
}

pub fn get_current_branch(repo: &Repository) -> Result<String, Error> {
    let head = repo.head()?;
    let branch = head
        .name()
        .ok_or_else(|| Error::from_str("Couldn't determine the branch name"))?;
    let branch_name = branch
        .strip_prefix("refs/heads/")
        .ok_or_else(|| Error::from_str("Invalid branch reference"))?;

    Ok(branch_name.to_string())
}

pub fn get_tree_for_commit<'a>(
    repo: &'a Repository,
    commit_hash: &str,
) -> Result<Tree<'a>, git2::Error> {
    let commit = repo.find_commit(repo.revparse_single(commit_hash)?.id())?;
    let tree = commit.tree()?;
    Ok(tree)
}

pub fn get_changed_files(
    repo: &Repository,
    old: &str,
    new: &str,
) -> Result<Vec<String>, git2::Error> {
    let old = get_tree_for_commit(repo, old)?;
    let new = get_tree_for_commit(repo, new)?;

    let mut diff_opts = DiffOptions::new();
    // diff_opts.pathspec(glob);
    diff_opts.force_binary(true);

    let diff = repo.diff_tree_to_tree(Some(&old), Some(&new), Some(&mut diff_opts))?;

    let mut changed_files = Vec::new();

    diff.foreach(
        &mut |delta, _progress| {
            if let Some(file_path) = delta.new_file().path().or_else(|| delta.old_file().path()) {
                let file_path_str = file_path.to_string_lossy().into_owned();
                if !changed_files.contains(&file_path_str) {
                    changed_files.push(file_path_str);
                }
            }
            true
        },
        None,
        None,
        None,
    )?;

    changed_files.sort();

    Ok(changed_files)
}
