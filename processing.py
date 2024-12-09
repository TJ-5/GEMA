import os
import csv
from logging_utils import log_error

def remove_extension(filename: str):
    parts = filename.split('.', 1)
    return parts[0]

def parse_track_filename(filename: str):
    original_base = remove_extension(filename)
    base = original_base.replace('_', ' ')
    tokens = base.split()

    def contains_digit(t):
        return any(ch.isdigit() for ch in t)
    
    def is_upper_token(t):
        letters = [c for c in t if c.isalpha()]
        if not letters:
            return False
        return t.isupper()

    state = 'BEFORE_DIGIT'
    index_tokens = []
    title_tokens = []
    artist_tokens = []
    
    for t in tokens:
        if state == 'BEFORE_DIGIT':
            index_tokens.append(t)
            if contains_digit(t):
                state = 'AFTER_DIGIT_BEFORE_TITLE'
        
        elif state == 'AFTER_DIGIT_BEFORE_TITLE':
            if is_upper_token(t):
                title_tokens.append(t)
                state = 'TITLE'
            else:
                index_tokens.append(t)
        
        elif state == 'TITLE':
            if is_upper_token(t):
                title_tokens.append(t)
            else:
                artist_tokens.append(t)
                state = 'ARTIST'
        
        else:  # ARTIST
            artist_tokens.append(t)
    
    index_str = '_'.join(index_tokens).strip().lower()
    title_str = ' '.join(title_tokens).strip().lower()
    artist_str = ' '.join(artist_tokens).strip().lower()
    
    return index_str, title_str, artist_str

def parse_duration(duration_str: str):
    duration_str = duration_str.replace(':', '.')
    parts = duration_str.split('.')
    
    if len(parts) < 2:
        return None
    
    main_part = parts[0]
    decimal_part = parts[1]
    number_str = main_part + '.' + decimal_part

    try:
        seconds = float(number_str)
        return seconds
    except ValueError:
        return None

def format_duration(seconds: float):
    total_hundredths = int(round(seconds * 100))
    s = total_hundredths // 100
    ms = total_hundredths % 100
    return f"{s}:{ms:02d}"

def load_labelcodes(labelcodes_file: str):
    label_dict = {}
    if not os.path.exists(labelcodes_file):
        return label_dict
    with open(labelcodes_file, 'r', encoding='utf-8') as f:
        lines = [l.strip() for l in f if l.strip()]
    for i in range(0, len(lines), 2):
        label = lines[i].strip()
        code = lines[i+1].strip() if i+1 < len(lines) else ''
        label_dict[label.lower()] = code
    return label_dict

def find_label_code(index_str: str, label_dict: dict):
    for label, code in label_dict.items():
        if index_str.startswith(label):
            return code
    return ''

def list_txt_files_in_dir(directory):
    files = []
    for root, dirs, filenames in os.walk(directory):
        for fn in filenames:
            if fn.lower().endswith('.txt'):
                files.append(os.path.join(root, fn))
    return files

def process_single_file(input_file, output_dir, label_dict, csv_columns):
    from logging_utils import log_error
    idx_title = artist_title = label_code_title = duration_title = None

    # Mapping von Spaltennamen zu Funktionen, um den Wert aus key oder total_seconds zu gewinnen
    # keys: (idx, title, artist, label_code)
    def get_column_value(col_name, key_tuple, total_seconds):
        idx, title, artist, label_code = key_tuple
        if col_name.lower() == "index":
            return idx
        elif col_name.lower() == "titel":
            return title
        elif col_name.lower() == "künstler":
            return artist
        elif col_name.lower() == "labelcode":
            return label_code
        elif col_name.lower() == "dauer":
            return format_duration(total_seconds)
        else:
            return ""  # Unbekannte Spalte

    track_dict = {}
    lines_read = 0
    lines_ignored_no_semicolon = 0
    lines_ignored_no_duration = 0
    lines_ignored_general = 0

    try:
        with open(input_file, 'r', encoding='utf-8') as infile:
            for line_num, line in enumerate(infile, start=1):
                line = line.strip()
                if not line:
                    continue
                lines_read += 1
                if ';' not in line:
                    lines_ignored_no_semicolon += 1
                    log_error(f"Datei {input_file}, Zeile {line_num}: Kein Semikolon.")
                    continue

                parts = line.split(';', 1)
                if len(parts) < 2:
                    lines_ignored_general += 1
                    log_error(f"Datei {input_file}, Zeile {line_num}: Unvollständige Zeile.")
                    continue
                
                filename = parts[0].strip()
                duration_str = parts[1].strip()

                idx, title, artist = parse_track_filename(filename)
                duration_in_seconds = parse_duration(duration_str)
                if duration_in_seconds is None:
                    lines_ignored_no_duration += 1
                    log_error(f"Datei {input_file}, Zeile {line_num}: Ungültige Dauer -> '{duration_str}'")
                    continue
                
                label_code = find_label_code(idx, label_dict)

                key = (idx, title, artist, label_code)
                if key in track_dict:
                    track_dict[key] += duration_in_seconds
                else:
                    track_dict[key] = duration_in_seconds
        
        base_name = os.path.basename(input_file)
        base_no_ext = remove_extension(base_name)
        output_file = os.path.join(output_dir, f"output_{base_no_ext}.csv")
        
        with open(output_file, 'w', newline='', encoding='utf-8') as outfile:
            writer = csv.writer(outfile, delimiter=';')
            writer.writerow(csv_columns)  # Spalten aus der Config
            for k, total_seconds in track_dict.items():
                row = [get_column_value(c, k, total_seconds) for c in csv_columns]
                writer.writerow(row)
        
        summary = (f"Datei '{input_file}':\n"
                   f"  Gelesene Zeilen: {lines_read}\n"
                   f"  Ignoriert (kein Semikolon): {lines_ignored_no_semicolon}\n"
                   f"  Ignoriert (ungültige Dauer): {lines_ignored_no_duration}\n"
                   f"  Ignoriert (allg. Fehler): {lines_ignored_general}\n"
                   f"  Ausgabe: {output_file}")
        
        log_error(summary)
        return summary
    except Exception as e:
        log_error("Exception: " + str(e))
        log_error(traceback.format_exc())
        return f"Fehler beim Verarbeiten von {input_file}: {e}"

