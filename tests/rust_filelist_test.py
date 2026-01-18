import importlib.util
import os
import sys
import types

import folder_paths


def load_module_from_path(module_name, module_path):
    spec = importlib.util.spec_from_file_location(module_name, module_path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def test_rust_scan_is_used_when_available(tmp_path, monkeypatch):
    rust_module_name = "comfyui_rust_filelist"
    module_path = os.path.join(
        os.path.dirname(__file__),
        "..",
        "__init__.py",
    )
    module_path = os.path.abspath(module_path)

    fake_rust = types.SimpleNamespace()
    fake_rust.calls = []

    def scan_model_folders(folders, extensions, excluded_dir_names):
        fake_rust.calls.append(
            {
                "folders": folders,
                "extensions": extensions,
                "excluded_dir_names": excluded_dir_names,
            }
        )
        return ["a.safetensors", "b.safetensors"], {str(tmp_path): 123.0}

    fake_rust.scan_model_folders = scan_model_folders
    monkeypatch.setitem(sys.modules, rust_module_name, fake_rust)

    test_folder_name = "rust_test_models"
    monkeypatch.setitem(
        folder_paths.folder_names_and_paths,
        test_folder_name,
        ([str(tmp_path)], {".safetensors"}),
    )

    original_get_filename_list_ = folder_paths.get_filename_list_
    try:
        load_module_from_path("comfyui_rust_filelist_test", module_path)
        result = folder_paths.get_filename_list_(test_folder_name)
        assert result[0] == ["a.safetensors", "b.safetensors"]
        assert fake_rust.calls[0]["folders"] == [str(tmp_path)]
    finally:
        folder_paths.get_filename_list_ = original_get_filename_list_
        folder_paths.filename_list_cache.clear()


def test_fallback_when_rust_missing(monkeypatch):
    rust_module_name = "comfyui_rust_filelist"
    module_path = os.path.join(
        os.path.dirname(__file__),
        "..",
        "__init__.py",
    )
    module_path = os.path.abspath(module_path)

    monkeypatch.delitem(sys.modules, rust_module_name, raising=False)

    original_get_filename_list_ = folder_paths.get_filename_list_
    try:
        load_module_from_path("comfyui_rust_filelist_test_missing", module_path)
        assert folder_paths.get_filename_list_ is original_get_filename_list_
    finally:
        folder_paths.get_filename_list_ = original_get_filename_list_
        folder_paths.filename_list_cache.clear()
