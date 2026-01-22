use crate::game_version_two::*;

#[cfg(test)]
mod tests {
    use super::*;
    use game::starling::units::Mass;

    #[test]
    fn consistent_masses() {
        for listing in RecipeListing::all() {
            let recipe = listing.to_recipe();

            if recipe.input_count() == 0 || recipe.output_count() == 0 {
                continue;
            }

            let mut input_mass = Mass::ZERO;
            for (item, count) in recipe.inputs() {
                input_mass += item.mass_per_unit() * count;
            }
            let mut output_mass = Mass::ZERO;
            for (item, count) in recipe.outputs() {
                output_mass += item.mass_per_unit() * count;
            }
            println!("{:?}, {}, {}", listing, input_mass, output_mass);
            assert_eq!(input_mass, output_mass);
        }
    }
}
