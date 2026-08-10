use serde::Deserialize;

use super::get_data;
use crate::models::{normalize_url, ApiError, VideoDetail, VideoPage};

#[derive(Deserialize)]
struct ViewOwner {
    name: String,
}

#[derive(Deserialize)]
struct ViewStat {
    view: u64,
}

#[derive(Deserialize)]
struct ViewPage {
    cid: u64,
    page: u32,
    #[serde(default)]
    part: String,
    duration: u64,
}

#[derive(Deserialize)]
struct ViewData {
    bvid: String,
    cid: u64,
    title: String,
    pic: String,
    duration: u64,
    owner: ViewOwner,
    stat: ViewStat,
    #[serde(default)]
    pages: Vec<ViewPage>,
}

pub async fn get_video_detail(bvid: &str) -> Result<VideoDetail, ApiError> {
    let data: ViewData = get_data(
        "https://api.bilibili.com/x/web-interface/view",
        &[("bvid", bvid)],
    )
    .await?;
    Ok(VideoDetail {
        bvid: data.bvid,
        cid: data.cid,
        title: data.title,
        cover: normalize_url(&data.pic),
        duration: data.duration,
        author: data.owner.name,
        play: data.stat.view,
        pages: data
            .pages
            .into_iter()
            .map(|page| VideoPage {
                cid: page.cid,
                page: page.page,
                part: page.part,
                duration: page.duration,
            })
            .collect(),
    })
}
