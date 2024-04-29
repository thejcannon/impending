import importlib.metadata
import os
import os.path
import sys

from ._impl import load_config


class MetaPathWrapper(list):
    on_spec_found: "list[OnSpecFoundMPF]" = []
    on_spec_not_found: "list[OnSpecNotFoundMPF]" = []

    def __super_iter__(self):
        return super().__iter__()

    def __iter__(self):
        return iter(
            (
                *self.on_spec_found,
                *self.__super_iter__(),
                *self.on_spec_not_found,
            )
        )


class OnSpecFoundMPF:
    def on_spec_found(self, spec):
        raise NotImplementedError()

    def find_spec(self, fullname, path, target=None):
        for finder in sys.meta_path.__super_iter__():
            if (spec := finder.find_spec(fullname, path, target)) is not None:
                if (spec := self.on_spec_found(spec)) is not None:
                    return spec
                # NB: Don't assume the same finder will work this time
                for finder in sys.meta_path.__super_iter__():
                    if spec := finder.find_spec(fullname, path, target):
                        return spec
        return None


class OnSpecNotFoundMPF:
    def on_spec_not_found(self, fullname, path, target):
        raise NotImplementedError()

    def find_spec(self, fullname, path, target=None):
        if self.on_spec_not_found(fullname, path, target):
            for finder in sys.meta_path.__super_iter__():
                if (spec := finder.find_spec(fullname, path, target)) is not None:
                    return spec
        return None


class RefreshPackageMPF(OnSpecFoundMPF):
    def __init__(self, config):
        self.config = config

    def on_spec_found(self, spec):
        try:
            # @TODO: This is semi inefficient, let's roll our own and also cache all dists
            dist = importlib.metadata.Distribution.from_name(spec.name)
        except importlib.metadata.PackageNotFoundError:
            return spec
        expected_version = self.config.get_expected_version(spec.name)
        if expected_version != dist.version:
            # NB: Since the core code loops over the real meta_path
            #   it won't make it to the InstallMissingPackageMPF.
            if self.config.install_missing_packages:
                self.config.maybe_install(spec.name)
            return None

        return spec


class InstallMissingPackageMPF(OnSpecNotFoundMPF):
    def __init__(self, config):
        self.config = config

    def on_spec_not_found(self, fullname, path, target):
        print(fullname, path, target, file=sys.stderr)
        print(sys.prefix)
        self.config.maybe_install(sys.prefix, fullname)
        return True


def _is_venv():
    return os.getenv("VIRTUAL_ENV") or os.path.exists(
        os.path.join(sys.prefix, "pyvenv.cfg")
    )


def install():
    if os.getenv("IMPENDING_NO_INSTALL") == "1":
        # @TODO: log
        return

    if not _is_venv():
        # @TODO: Log
        return

    config = load_config()
    if config.enforce_package_versions:
        MetaPathWrapper.on_spec_found.append(RefreshPackageMPF(config))
    if config.install_missing_packages:
        MetaPathWrapper.on_spec_not_found.append(InstallMissingPackageMPF(config))

    # @TODO: I don't understand the double-call :(
    os.environ["IMPENDING_NO_INSTALL"] = "1"


sys.meta_path = MetaPathWrapper(sys.meta_path)
__all__ = ["install", "MetaPathWrapper"]
