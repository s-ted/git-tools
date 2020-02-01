use crate::git::Git;
use crate::Fork;

pub fn run(params: Fork) -> Result<(), Box<dyn std::error::Error>> {
    let mut git = Git::open()?;

    let branch_name = params.branch_name.as_str();

    let mut message = "Initial commit\n\n".to_string();

    if let Some(name) = params.from.as_ref() {
        match git.get_branch_hash(name.as_str())? {
            // name is really a branch
            Some(hash) => {
                message.push_str(&format!("Forked at: {}\n", hash));
                message.push_str(&format!("Parent branch: {}\n", name));

                git.branch(branch_name, Some(hash.as_str()))?;
            }
            // name was not a branch
            None => {
                message.push_str(&format!("Forked at: {}\n", name));
                message.push_str(&format!("No parent branch.\n"));

                git.branch(branch_name, Some(name))?;
            }
        }
    } else if let Some(name) = git.branch_name.as_ref() {
        message.push_str(&format!("Forked at: {}\n", git.head_hash));
        message.push_str(&format!("Parent branch: {}\n", name));

        git.branch(branch_name, None)?;
    } else {
        message.push_str(&format!("Forked at: {}\n", git.head_hash));
        message.push_str(&format!("No parent branch.\n"));

        git.branch(branch_name, None)?;
    }

    git.switch_branch(branch_name)?;
    git.commit(message.as_str())?;

    println!("Branch {} created.", branch_name);

    Ok(())
}
