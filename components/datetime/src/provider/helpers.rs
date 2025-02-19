// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use crate::date;
use crate::error::DateTimeFormatError;
use crate::fields;
use crate::options::{components, length, preferences, DateTimeFormatOptions};
use crate::pattern::{Pattern, PatternItem};
use crate::provider;
use crate::skeleton;
use alloc::borrow::Cow;

type Result<T> = core::result::Result<T, DateTimeFormatError>;

pub trait DateTimePatterns {
    fn get_pattern_for_options(&self, options: &DateTimeFormatOptions) -> Result<Option<Pattern>>;
    fn get_pattern_for_components_bag(
        &self,
        components: &components::Bag,
    ) -> Result<Option<Pattern>>;
    fn get_pattern_for_length_bag(&self, length: &length::Bag) -> Result<Option<Pattern>>;
    fn get_pattern_for_date_length(&self, length: length::Date) -> Result<Pattern>;
    fn get_pattern_for_time_length(
        &self,
        length: length::Time,
        preferences: &Option<preferences::Bag>,
    ) -> Result<Pattern>;
    fn get_pattern_for_datetime_length(
        &self,
        length: length::Date,
        date: Pattern,
        time: Pattern,
    ) -> Result<Pattern>;
}

pub trait DateTimeSymbols {
    fn get_symbol_for_month(
        &self,
        month: fields::Month,
        length: fields::FieldLength,
        num: usize,
    ) -> &Cow<str>;
    fn get_symbol_for_weekday(
        &self,
        weekday: fields::Weekday,
        length: fields::FieldLength,
        day: date::IsoWeekday,
    ) -> &Cow<str>;
    fn get_symbol_for_day_period(
        &self,
        day_period: fields::DayPeriod,
        length: fields::FieldLength,
        hour: date::IsoHour,
        is_top_of_hour: bool,
    ) -> &Cow<str>;
}

impl DateTimePatterns for provider::gregory::DatePatternsV1 {
    fn get_pattern_for_options(&self, options: &DateTimeFormatOptions) -> Result<Option<Pattern>> {
        match options {
            DateTimeFormatOptions::Length(bag) => self.get_pattern_for_length_bag(bag),
            DateTimeFormatOptions::Components(bag) => self.get_pattern_for_components_bag(bag),
        }
    }

    fn get_pattern_for_components_bag(
        &self,
        components: &components::Bag,
    ) -> Result<Option<Pattern>> {
        // Not all skeletons are currently supported.
        let requested_fields = components.to_vec_fields();
        Ok(
            match skeleton::create_best_pattern_for_fields(
                &self.datetime.skeletons,
                &self.datetime.length_patterns,
                &requested_fields,
                components,
                false, // Prefer the requested fields over the matched pattern.
            ) {
                skeleton::BestSkeleton::AllFieldsMatch(pattern)
                | skeleton::BestSkeleton::MissingOrExtraFields(pattern) => Some(pattern),
                skeleton::BestSkeleton::NoMatch => None,
            },
        )
    }

    fn get_pattern_for_length_bag(&self, length: &length::Bag) -> Result<Option<Pattern>> {
        match (length.date, length.time) {
            (None, None) => Ok(None),
            (None, Some(time_length)) => self
                .get_pattern_for_time_length(time_length, &length.preferences)
                .map(Some),
            (Some(date_length), None) => self.get_pattern_for_date_length(date_length).map(Some),
            (Some(date_length), Some(time_length)) => {
                let time = self.get_pattern_for_time_length(time_length, &length.preferences)?;
                let date = self.get_pattern_for_date_length(date_length)?;

                self.get_pattern_for_datetime_length(date_length, date, time)
                    .map(Some)
            }
        }
    }

    fn get_pattern_for_date_length(&self, length: length::Date) -> Result<Pattern> {
        let date = &self.date;
        let s = match length {
            length::Date::Full => &date.full,
            length::Date::Long => &date.long,
            length::Date::Medium => &date.medium,
            length::Date::Short => &date.short,
        };
        Ok(Pattern::from_bytes(s)?)
    }

    fn get_pattern_for_datetime_length(
        &self,
        length: length::Date,
        date: Pattern,
        time: Pattern,
    ) -> Result<Pattern> {
        let datetime = &self.datetime;
        let s = match length {
            length::Date::Full => &datetime.length_patterns.full,
            length::Date::Long => &datetime.length_patterns.long,
            length::Date::Medium => &datetime.length_patterns.medium,
            length::Date::Short => &datetime.length_patterns.short,
        };
        Ok(Pattern::from_bytes_combination(s, date, time)?)
    }

    /// Look up the proper pre-computed pattern for a given length. If a preference for an hour
    /// cycle is set, it will look look up a pattern in the time_h11_12 or time_h23_h24 provider
    /// data, and then manually modify the symbol in the pattern if needed.
    fn get_pattern_for_time_length(
        &self,
        length: length::Time,
        preferences: &Option<preferences::Bag>,
    ) -> Result<Pattern> {
        // Determine the coarse hour cycle patterns to use from either the preference bag,
        // or the preferred hour cycle for the locale.
        let time = if let Some(preferences::Bag {
            hour_cycle: Some(hour_cycle_pref),
        }) = preferences
        {
            match hour_cycle_pref {
                preferences::HourCycle::H11 | preferences::HourCycle::H12 => &self.time_h11_h12,
                preferences::HourCycle::H23 | preferences::HourCycle::H24 => &self.time_h23_h24,
            }
        } else {
            match self.preferred_hour_cycle {
                crate::pattern::CoarseHourCycle::H11H12 => &self.time_h11_h12,
                crate::pattern::CoarseHourCycle::H23H24 => &self.time_h23_h24,
            }
        };

        let mut pattern = Pattern::from_bytes(match length {
            length::Time::Full => &time.full,
            length::Time::Long => &time.long,
            length::Time::Medium => &time.medium,
            length::Time::Short => &time.short,
        })?;

        if let Some(preferences::Bag {
            hour_cycle: Some(hour_cycle_pref),
        }) = preferences
        {
            // Apply the preference::Bag override and change the pattern from a coarse hour cycle
            // to the specific hour cycle.
            for item in pattern.items_mut() {
                if let PatternItem::Field(fields::Field { symbol, length: _ }) = item {
                    if let fields::FieldSymbol::Hour(_) = symbol {
                        *symbol = fields::FieldSymbol::Hour(hour_cycle_pref.field());
                    }
                }
            }
        }

        Ok(pattern)
    }
}

impl DateTimeSymbols for provider::gregory::DateSymbolsV1 {
    fn get_symbol_for_weekday(
        &self,
        weekday: fields::Weekday,
        length: fields::FieldLength,
        day: date::IsoWeekday,
    ) -> &Cow<str> {
        let widths = match weekday {
            fields::Weekday::Format => &self.weekdays.format,
            fields::Weekday::StandAlone => {
                if let Some(ref widths) = self.weekdays.stand_alone {
                    let symbols = match length {
                        fields::FieldLength::Wide => widths.wide.as_ref(),
                        fields::FieldLength::Narrow => widths.narrow.as_ref(),
                        fields::FieldLength::Six => widths
                            .short
                            .as_ref()
                            .or_else(|| widths.abbreviated.as_ref()),
                        _ => widths.abbreviated.as_ref(),
                    };
                    if let Some(symbols) = symbols {
                        return &symbols.0[(day as usize) % 7];
                    } else {
                        return self.get_symbol_for_weekday(fields::Weekday::Format, length, day);
                    }
                } else {
                    return self.get_symbol_for_weekday(fields::Weekday::Format, length, day);
                }
            }
            fields::Weekday::Local => unimplemented!(),
        };
        let symbols = match length {
            fields::FieldLength::Wide => &widths.wide,
            fields::FieldLength::Narrow => &widths.narrow,
            fields::FieldLength::Six => widths.short.as_ref().unwrap_or(&widths.abbreviated),
            _ => &widths.abbreviated,
        };
        &symbols.0[(day as usize) % 7]
    }

    fn get_symbol_for_month(
        &self,
        month: fields::Month,
        length: fields::FieldLength,
        num: usize,
    ) -> &Cow<str> {
        // TODO(#493): Support symbols for non-Gregorian calendars.
        debug_assert!(num < 12);
        let widths = match month {
            fields::Month::Format => &self.months.format,
            fields::Month::StandAlone => {
                if let Some(ref widths) = self.months.stand_alone {
                    let symbols = match length {
                        fields::FieldLength::Wide => widths.wide.as_ref(),
                        fields::FieldLength::Narrow => widths.narrow.as_ref(),
                        _ => widths.abbreviated.as_ref(),
                    };
                    if let Some(symbols) = symbols {
                        return &symbols.0[num];
                    } else {
                        return self.get_symbol_for_month(fields::Month::Format, length, num);
                    }
                } else {
                    return self.get_symbol_for_month(fields::Month::Format, length, num);
                }
            }
        };
        let symbols = match length {
            fields::FieldLength::Wide => &widths.wide,
            fields::FieldLength::Narrow => &widths.narrow,
            _ => &widths.abbreviated,
        };
        &symbols.0[num]
    }

    fn get_symbol_for_day_period(
        &self,
        day_period: fields::DayPeriod,
        length: fields::FieldLength,
        hour: date::IsoHour,
        is_top_of_hour: bool,
    ) -> &Cow<str> {
        use fields::{DayPeriod::NoonMidnight, FieldLength};
        let widths = &self.day_periods.format;
        let symbols = match length {
            FieldLength::Wide => &widths.wide,
            FieldLength::Narrow => &widths.narrow,
            _ => &widths.abbreviated,
        };
        match (day_period, u8::from(hour), is_top_of_hour) {
            (NoonMidnight, 00, true) => symbols.midnight.as_ref().unwrap_or(&symbols.am),
            (NoonMidnight, 12, true) => symbols.noon.as_ref().unwrap_or(&symbols.pm),
            (_, hour, _) if hour < 12 => &symbols.am,
            _ => &symbols.pm,
        }
    }
}
