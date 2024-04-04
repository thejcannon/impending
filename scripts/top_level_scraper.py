import pathlib
import requests
import json
import logging
from bs4 import BeautifulSoup
from collections import defaultdict

from pip._internal.models.wheel import Wheel
from pip._vendor.packaging.utils import canonicalize_name
from pip._internal.network.lazy_wheel import dist_from_wheel_url
from pip._internal.models.link import Link
from pip._internal.operations.prepare import PipSession
from pip._internal.exceptions import UnsupportedWheel

session  = PipSession()
def _fetch_metadata_using_lazy_wheel(
    wheel_url,
):
    link = Link(wheel_url)
    if link.is_file or not link.is_wheel:
        return None
    try:
        wheel = Wheel(link.filename)
    except Exception:  # Not a valid wheel name?
        return None
    name = canonicalize_name(wheel.name)
    url = link.url.split("#", 1)[0]
    try:
        return dist_from_wheel_url(name, url, session)
    except UnsupportedWheel:
        return None

mega_list = requests.get(
    "https://gist.githubusercontent.com/charliermarsh/07afd9f543dfea68408a4a42cede4be4/raw/6639cd58a2e10d6bb7821f891f00322c8630b60a/pypi_10k_most_dependents.txt"
)

logger = logging.getLogger(__name__)


whl_to_top_level = defaultdict(dict)
with open("whl_to_top_level.json") as f:
    whl_to_top_level.update(json.load(f))

whl_to_top_level.setdefault("!!!!!", [])
seen = set(whl_to_top_level['!!!!!'])

try:
    for pkgname in mega_list.text.splitlines():
        if pkgname in seen:
            continue
        print(f"Looking up {pkgname}")
        response = requests.get(
            f"https://pypi.org/simple/{pkgname}"
        )
        soup = BeautifulSoup(response.text, "html.parser")
        whl_links = [link.get("href") for link in soup.find_all("a", href=True) if link.get("href").rsplit("#", 1)[0].endswith(".whl")]
        print(f"  Found {len(whl_links)} wheels")
        whl_links = [
            link
            for link in whl_links
            if ("cp36" not in link and "cp27" not in link and "cp37" not in link and "cp35" not in link and "cp34" not in link)
        ]
        if whl_links:
            link = whl_links[-1]

            if pkgname in whl_to_top_level:
                continue

            print(f"  Fetching metadata for {link}")
            d = _fetch_metadata_using_lazy_wheel(link)
            if d is None:
                whl_to_top_level[pkgname] = []
                continue
            try:
                whl_to_top_level[pkgname] = d.read_text("top_level.txt").splitlines()
            except FileNotFoundError:
                whl_to_top_level[pkgname] = []

            print(f"Finished with {pkgname}")
        else:
            whl_to_top_level[pkgname] = []
        seen.add(pkgname)
finally:
    whl_to_top_level['!!!!!'] = sorted(seen)
    with open("whl_to_top_level.json", "w") as f:
        json.dump(whl_to_top_level, f)
