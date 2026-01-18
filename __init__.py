import importlib
import logging
import os
import sys
import time
from typing import Iterable

import folder_paths

_LOGGER = logging.getLogger(__name__)

_RUST_MODULE_NAME = "comfyui_rust_filelist"


def _load_rust() -> bool:
    if _RUST_MODULE_NAME in sys.modules:
        return True
    module_dir = os.path.dirname(__file__)
    if module_dir not in sys.path:
        sys.path.insert(0, module_dir)
    try:
        importlib.import_module(_RUST_MODULE_NAME)
        return True
    except Exception as exc:
        _LOGGER.warning("[rust-filelist] Rust module unavailable, fallback to Python scan: %s", exc)
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
            _LOGGER.warning("[rust-filelist] Rust scan failed, fallback to Python: %s", exc)
            return original_get_filename_list_(folder_name)

    folder_paths.get_filename_list_ = get_filename_list_rust

    _LOGGER.info("[rust-filelist] Rust scan enabled for model folders")


try:
    _patch_get_filename_list()
except Exception as exc:
    _LOGGER.warning("[rust-filelist] Failed to patch get_filename_list_: %s", exc)

NODE_CLASS_MAPPINGS = {}
NODE_DISPLAY_NAME_MAPPINGS = {}
