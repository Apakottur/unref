from models import Book, Film, Movie, Options, Point


def show(movie: Movie, options: Options, point: Point, film: Film, book: Book) -> None:
    print(movie["title"], movie.get("year"), movie.pop("rating"), movie.setdefault("studio", ""))
    print("budget" in movie, "country" not in movie, options["verbose"], point["x"], film["runtime"])
    match movie:
        case {"language": language}:
            print(language)
    movie["director"] = "Scott"
    print(movie.genre, book.author)


# Other subscripts, calls and comparisons read no key.
def other(values: dict[str, int], key: str) -> None:
    print(values[0], values.get(key), values.get(), "title" == key, 1 in values)


def unused(movie: Movie) -> str:  # expect: 5 [unused-function]
    return movie["sequel"]


show(
    {"title": "Blade Runner", "year": 1982, "director": "Scott"},
    Options(verbose=True, quiet=False),
    {"x": 1},
    Film(title="Alien", runtime=117),
    Book(),
)
other({}, "")
