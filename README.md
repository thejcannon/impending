# Impending

`impending` is a library used to provide "incremental dependency resolution".

Although there are several "modes", at it's core `impending` exists to help you
run your code without worrying about installing dependencies (or what versions to install).

# How does it work?

After you create a new virtual environment, install `impending` (`impending` has 0 Python dependencies).

Then, as you `import` modules, depending on the mode (documented below) `impending` will:
- Find the relevant package for the module
- If it's already installed, optionally do some version checking
- If it needs to be installed, install it
- Load the module

# Modes
