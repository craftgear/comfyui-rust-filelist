import importlib
import logging
import os
import sys
import time
from typing import Iterable

import folder_paths

_LOGGER = logging.getLogger(__name__)

_RUST_MODULE_NAME = "comfyui_fast_filelist"


def _rust_search_paths(module_dir: str, platform: str | None = None) -> list[str]:
    platform_value = platform or sys.platform
    bin_dir = os.path.join(module_dir, "bin")
    platform_dir = None
    if platform_value.startswith("darwin"):
        platform_dir = os.path.join(bin_dir, "macos")
    elif platform_value.startswith("win"):
        platform_dir = os.path.join(bin_dir, "windows")
    elif platform_value.startswith("linux"):
        platform_dir = os.path.join(bin_dir, "linux")

    if platform_dir:
        return [platform_dir, bin_dir, module_dir]
    return [bin_dir, module_dir]


def _load_rust() -> bool:
    if _RUST_MODULE_NAME in sys.modules:
        return True
    module_dir = os.path.dirname(__file__)
    for path in _rust_search_paths(module_dir):
        if path not in sys.path:
            sys.path.insert(0, path)
    try:
        importlib.import_module(_RUST_MODULE_NAME)
        return True
    except Exception as exc:
        _LOGGER.warning("[fast-filelist] Rust module unavailable, fallback to Python scan: %s", exc)
        return False


def _to_list(values: Iterable[str]) -> list[str]:
    return [v for v in values]


def _patch_get_filename_list() -> None:
    if not _load_rust():
        return

    rust = importlib.import_module(_RUST_MODULE_NAME)
    original_get_filename_list_ = folder_paths.get_filename_list_

    def get_filename_list_rust(folder_name: str):
        try:
            folder_name = folder_paths.map_legacy(folder_name)
            folders = folder_paths.folder_names_and_paths[folder_name]
            folders_list = _to_list(folders[0])
            extensions = _to_list(folders[1])
            excluded_dir_names = [".git"]
            files, folders_all = rust.scan_model_folders(folders_list, extensions, excluded_dir_names)
            return sorted(list(files)), folders_all, time.perf_counter()
        except Exception as exc:
            _LOGGER.warning("[fast-filelist] Rust scan failed, fallback to Python: %s", exc)
            return original_get_filename_list_(folder_name)

    folder_paths.get_filename_list_ = get_filename_list_rust

    _LOGGER.info("[fast-filelist] Rust scan enabled for model folders")


try:
    _patch_get_filename_list()
except Exception as exc:
    _LOGGER.warning("[fast-filelist] Failed to patch get_filename_list_: %s", exc)

NODE_CLASS_MAPPINGS = {}
NODE_DISPLAY_NAME_MAPPINGS = {}
