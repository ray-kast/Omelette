enum SortRange {
  Hour,
  Day,
  Week,
  Month,
  Year,
  All,
}

enum SortType {
  Hot,
  New,
  Rising,
  Top(SortRange),
  Controversial(SortRange),
}

impl ToString for SortRange {
  fn to_string(&self) -> String {
    match self {
      SortRange::Hour => "hour",
      SortRange::Day => "day",
      SortRange::Week => "week",
      SortRange::Month => "month",
      SortRange::Year => "year",
      SortRange::All => "all",
    }.to_string()
  }
}

impl ToString for SortType {
  fn to_string(&self) -> String {
    match self {
      SortType::Hot => "hot",
      SortType::New => "new",
      SortType::Rising => "rising",
      SortType::Top(_) => "top",
      SortType::Controversial(_) => "controversial",
    }.to_string()
  }
}
