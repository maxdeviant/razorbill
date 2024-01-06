pub struct SectionToRender<'a> {
    pub title: &'a Option<String>,
    pub path: &'a str,
    pub raw_content: &'a str,
    pub pages: Vec<PageToRender<'a>>,
}

pub struct PageToRender<'a> {
    pub title: &'a Option<String>,
    pub slug: &'a str,
    pub path: &'a str,
    pub raw_content: &'a str,
}
