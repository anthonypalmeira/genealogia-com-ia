use super::{Date, GenderType, Lifespan, impossible_together};

fn allowed(birth: Date, parent: Lifespan<'_>, gender: Option<&GenderType>) -> bool {
    let child = Lifespan {
        name: "Filho",
        born: Some(birth),
        died: None,
    };
    impossible_together(child, parent, gender).is_empty()
}

#[test]
fn age_limits_include_both_endpoints() {
    let parent = Lifespan {
        name: "Pessoa",
        born: Some(Date::of_year(1800)),
        died: None,
    };
    for (year, mother, other) in [
        (1811, false, false),
        (1812, true, true),
        (1855, true, true),
        (1856, false, true),
        (1880, false, true),
        (1881, false, false),
    ] {
        let birth = Date::of_year(year);
        assert_eq!(
            allowed(birth, parent, Some(&GenderType::Female)),
            mother,
            "{year}"
        );
        for gender in [Some(&GenderType::Male), None] {
            assert_eq!(allowed(birth, parent, gender), other, "{year}, {gender:?}");
        }
    }
}

#[test]
fn a_birth_after_death_is_compared_at_the_precision_both_dates_have() {
    let date = |year, month, day| Date {
        year,
        month: Some(month),
        day,
    };
    let death = date(1850, 5, Some(10));
    let parent = Lifespan {
        name: "Pessoa",
        born: None,
        died: Some(death),
    };
    let mother = Some(&GenderType::Female);
    let next_day = date(1850, 5, Some(11));
    assert!(allowed(death, parent, mother));
    assert!(!allowed(next_day, parent, mother));
    assert!(allowed(date(1850, 5, None), parent, mother));
    assert!(!allowed(date(1850, 6, None), parent, mother));

    let year_only = Lifespan {
        died: Some(Date::of_year(1850)),
        ..parent
    };
    assert!(allowed(next_day, year_only, mother));
    assert!(!allowed(Date::of_year(1851), year_only, mother));
    for gender in [Some(&GenderType::Male), None] {
        assert!(allowed(date(1851, 5, Some(10)), parent, gender));
        assert!(!allowed(date(1851, 5, Some(11)), parent, gender));
    }
}
