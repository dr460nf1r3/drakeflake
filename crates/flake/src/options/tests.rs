use drakeflake_render::OptionName;

use super::*;

#[test]
fn wanted_options_are_relative_to_their_tree() {
    let names = [
        "home-manager.users.nico.programs.git.enable".to_owned(),
        "home-manager.users.other.programs.git.enable".to_owned(),
        "services.openssh.enable".to_owned(),
    ];

    let locs = |tree: &OptionTree, asked: &[String]| {
        let asked: Vec<OptionName> = asked.iter().map(OptionName::from).collect();
        relative_wanted(tree, &asked)
            .into_iter()
            .map(|wanted| (wanted.name.to_owned(), wanted.loc))
            .collect::<Vec<_>>()
    };

    assert_eq!(
        locs(&OptionTree::HomeManager("nico".into()), &names),
        [
            (names[0].clone(), vec!["programs", "git", "enable"]),
            (names[1].clone(), vec![]),
            (names[2].clone(), vec![]),
        ]
        .map(|(name, loc)| (name, loc.into_iter().map(str::to_owned).collect::<Vec<_>>()))
    );
    assert_eq!(
        locs(&OptionTree::System, &names[2..]),
        [(
            names[2].clone(),
            vec![
                "services".to_owned(),
                "openssh".to_owned(),
                "enable".to_owned()
            ]
        )]
    );

    let dotted = ["home-manager.users.\"first.last\".home.stateVersion".to_owned()];
    assert_eq!(
        locs(&OptionTree::HomeManager("first.last".into()), &dotted),
        [(
            dotted[0].clone(),
            vec!["home".to_owned(), "stateVersion".to_owned()]
        )]
    );
}
