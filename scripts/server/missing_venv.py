# Loaded by viewfinder-kernel when the user's backend venv doesn't exist yet.
# Replaces every cell with an error that says how to create the venv.
import os

_msg = (
    "The viewfinder backend venv was not found. Looked in:\n"
    + "".join(f"    {p}\n" for p in os.environ.get("VIEWFINDER_MISSING_VENV", "").split())
    + "Set it up once from a terminal:\n"
    "    git clone https://github.com/cau777/viewfinder-scaffold.git ~/viewfinder-scaffold\n"
    "    cd ~/viewfinder-scaffold && make setup\n"
    "then restart this kernel (Kernel > Restart Kernel)."
)
get_ipython().input_transformers_cleanup.append(  # noqa: F821
    lambda lines: [f"raise RuntimeError({_msg!r})\n"]
)
