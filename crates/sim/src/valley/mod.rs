//! I-052 block valley game layer (pack, crafting, quest, invention).
//! Tests first: the types land in the next commit.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tier::Tier;

    // -----------------------------------------------------------------------
    // Contract step (section 10, item 5): green from the first merge.
    // -----------------------------------------------------------------------

    #[test]
    fn valley_contract_types_exist() {
        let p = Pack::default();
        assert_eq!(p.slots.len(), 20);
        assert!(p.slots.iter().all(Option::is_none));
        assert_eq!(p.hot, [None; 4]);
        let _ = Stack {
            item: Item::Block(7),
            n: 1,
        };
        for it in [
            Item::Pick,
            Item::Torch,
            Item::O2Flask,
            Item::H2Balloon,
            Item::KilnSpark,
        ] {
            let _ = Stack { item: it, n: 1 };
        }
        let _: &[Recipe] = RECIPES;
        let q = Quest::new();
        assert_eq!(q.step(), 0, "the quest starts at the intro");
        assert_eq!(STEPS, 7, "steps 1-7 after the intro (section 7)");
        let _ = |q: &mut Quest| q.on(QuestEvent::IntroDone);
        let _ = |q: &Quest| q.line(Some(Tier::Seedling)).len() + q.line(None).len();
        let _ = |b: Tier| invent::judge(&[Item::Block(15), Item::O2Flask], b, "");
    }

    // -----------------------------------------------------------------------
    // Acceptance (section 11, C1-C6). Bodies by Lane C (spec section 12);
    // each ignored body is `todo!()`, so un-ignoring without one fails.
    // -----------------------------------------------------------------------

    /// C1: 64 stack limit, leftover on full, `take` is all-or-nothing.
    #[test]
    #[ignore = "Lane C (Sun night): body with Pack"]
    fn pack_stacks_and_overflows() {
        todo!("C1")
    }

    /// C2: `craft("torch")` fails with `NotEnough{coal}` and leaves the pack
    /// unchanged; with both it gives 4 torches.
    #[test]
    #[ignore = "Lane C (Sun night): body with craft"]
    fn torch_needs_wood_and_coal() {
        todo!("C2")
    }

    /// C3: events out of order do not skip steps; repeating an event does not
    /// complete a step twice.
    #[test]
    #[ignore = "Lane C (Sun night): body with the quest table"]
    fn quest_advances_only_in_order() {
        todo!("C3")
    }

    /// C4: `{Coal, O2Flask}` -> `carbon_combustion`, relights; `{H2Balloon,
    /// O2Flask}` -> `water_synthesis`, relights; `{Coal}` -> fizzle "needs
    /// oxygen"; `{water}` -> fizzle "nothing to burn"; typed `C + O2 -> CO2` at
    /// Apprentice is accepted; typed `C + O2 -> CO` is refused with the
    /// existing `atoms_off` code and its reason (unbalanced as written; the
    /// reason must not call carbon monoxide impossible).
    #[test]
    #[ignore = "Lane C (Tue): body with the Invention Bench"]
    fn invention_judges_real_reactions() {
        todo!("C4")
    }

    /// C5: Fire at Sprout is usable while unknown; Frost at Seedling is usable
    /// only from Learning.
    #[test]
    #[ignore = "Lane C (Sun night): body with usable"]
    fn usable_follows_band() {
        todo!("C5")
    }

    /// C6: every step has a line for all four bands and for no band, at most
    /// 90 characters.
    #[test]
    #[ignore = "Lane C (Sun night): body with the quest lines"]
    fn quest_lines_exist_per_band() {
        todo!("C6")
    }
}
