use std::error::Error;
use std::fs;

use chrono::{DateTime, Local, TimeZone};
use chrono_tz::Tz;
use icalendar::{Calendar, CalendarDateTime, Component, DatePerhapsTime, EventLike};
use windows_timezones::WindowsTimezone;

// Our own model: plain owned strings and local times, no parser types leak past this point.
// `pub` makes an item visible outside this module; everything is private by default.
pub struct Event {
    pub summary: String,
    // Option = may be absent. None replaces null.
    pub start: Option<DateTime<Local>>,
    pub end: Option<DateTime<Local>>,
    pub all_day: bool,
    pub location: Option<String>,
    pub description: Option<String>,
}

// Functions attached to the type. `Self` means Event.
impl Event {
    // No `self` parameter: called as Event::from_ical(...), like a constructor.
    fn from_ical(event: &icalendar::Event) -> Self {
        let start = event.get_start();
        let all_day = matches!(start, Some(DatePerhapsTime::Date(_)));
        Event {
            summary: event.get_summary().unwrap_or("(no title)").to_string(),
            // and_then flattens: to_local already returns an Option.
            start: start.and_then(to_local),
            end: event.get_end().and_then(to_local),
            all_day,
            location: event.get_location().map(str::to_string),
            description: event.get_description().map(str::to_string),
        }
    }

    // `&self`: a method that only reads the instance.
    pub fn format_time(&self, time: Option<DateTime<Local>>) -> String {
        let Some(time) = time else {
            return "(none)".to_string();
        };
        if self.all_day {
            time.format("%Y-%m-%d (all day)").to_string()
        } else {
            time.format("%Y-%m-%d %H:%M").to_string()
        }
    }
}

pub fn load_events(path: &str) -> Result<Vec<Event>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let calendar: Calendar = content.parse()?;
    // Lazy iterator chain: nothing runs until collect() pulls.
    let mut events: Vec<Event> = calendar
        .components
        .iter()
        // Keeps only Some(...) results, so todos and venues are dropped.
        .filter_map(|component| component.as_event())
        .map(Event::from_ical)
        .collect();
    // `mut` above is required because sorting modifies the Vec in place.
    events.sort_by_key(|event| event.start);
    Ok(events)
}

// Every arm must be covered: match is exhaustive, the compiler checks it.
fn to_local(date: DatePerhapsTime) -> Option<DateTime<Local>> {
    match date {
        DatePerhapsTime::Date(day) => Local.from_local_datetime(&day.and_hms_opt(0, 0, 0)?).single(),
        DatePerhapsTime::DateTime(CalendarDateTime::Floating(naive)) => {
            Local.from_local_datetime(&naive).single()
        }
        DatePerhapsTime::DateTime(CalendarDateTime::Utc(utc)) => Some(utc.with_timezone(&Local)),
        DatePerhapsTime::DateTime(CalendarDateTime::WithTimezone { date_time, tzid }) => {
            let tz = resolve_tz(&tzid)?;
            // single(): None if the wall-clock time is ambiguous or missing (DST switch).
            let zoned = tz.from_local_datetime(&date_time).single()?;
            Some(zoned.with_timezone(&Local))
        }
    }
}

// Try an IANA name first, then fall back to the Windows name table.
fn resolve_tz(tzid: &str) -> Option<Tz> {
    tzid.parse::<Tz>()
        .ok()
        .or_else(|| tzid.parse::<WindowsTimezone>().ok().map(Tz::from))
}
