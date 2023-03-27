use git2::Repository;

fn main() {

    let repo = Repository::discover(".").unwrap();
    dbg!(repo.head().unwrap().name());

}
