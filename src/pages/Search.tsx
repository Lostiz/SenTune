import { SearchBar } from "../components/search/SearchBar";
import { SearchResults } from "../components/search/SearchResults";

export function SearchPage() {
  return (
    <section className="page search-page">
      <p className="page__subtitle">搜索</p>
      <h1>搜索</h1>
      <SearchBar />
      <SearchResults />
    </section>
  );
}
