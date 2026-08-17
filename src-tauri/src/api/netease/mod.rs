//! 网易云音乐源（无登录）：公开接口搜索与播放地址解析。
//!
//! 说明：weapi 加密接口已失效（2026 年实测返回空响应），改用无需加密的
//! 公开接口（`/api/cloudsearch/pc`、`/api/song/enhance/player/url`）。
//! 抗风控策略（限流 / 退避 / 熔断 / Cookie 稳定）集中在 `client.rs`。

pub(crate) mod artist;
pub(crate) mod client;
mod lyric;
mod playurl;
mod search;

pub use artist::{
    get_album_detail, get_artist_albums, get_artist_detail, get_artist_songs, search_artists,
};
pub use lyric::get_lyric;
pub use playurl::get_play_url;
pub use search::search;
