import json
import os

CONFIG_FILE = 'config.json'

def load_config():
    if not os.path.exists(CONFIG_FILE):
        # Standard-Config erstellen
        default_config = {
            "labelcodes_file": "Labelcodes.txt",
            "default_output_dir": ".",
            "csv_columns": ["Index", "Titel", "Künstler", "Labelcode", "Dauer"]
        }
        with open(CONFIG_FILE, 'w', encoding='utf-8') as f:
            json.dump(default_config, f, indent=2)
        return default_config
    else:
        with open(CONFIG_FILE, 'r', encoding='utf-8') as f:
            return json.load(f)
