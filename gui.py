import sys
import traceback
from PyQt5.QtWidgets import (QWidget, QLabel, QVBoxLayout, QPushButton, QListWidget,
                             QFileDialog, QProgressBar, QHBoxLayout)
from PyQt5.QtCore import Qt

from config import load_config
from processing import load_labelcodes, list_txt_files_in_dir, process_single_file
from logging_utils import log_error

class DragDropWindow(QWidget):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Track Parser")
        self.setAcceptDrops(True)
        
        self.config = load_config()
        self.output_dir = self.config.get("default_output_dir", ".")
        self.labelcodes_file = self.config.get("labelcodes_file", "Labelcodes.txt")
        self.csv_columns = self.config.get("csv_columns", ["Index", "Titel", "Künstler", "Labelcode", "Dauer"])
        self.label_dict = load_labelcodes(self.labelcodes_file)
        
        # Obere Button-Leiste
        self.output_button = QPushButton("Ausgabeort wählen", self)
        self.output_button.setToolTip("Wähle den Ordner für die CSV-Ausgabe.")
        self.output_button.clicked.connect(self.choose_output_directory)
        
        self.reload_button = QPushButton("Labelcodes neu laden", self)
        self.reload_button.setToolTip("Lade die Labelcodes neu.")
        self.reload_button.clicked.connect(self.reload_labelcodes)
        
        self.file_select_button = QPushButton("Datei auswählen", self)
        self.file_select_button.setToolTip("Wähle .txt-Dateien aus.")
        self.file_select_button.clicked.connect(self.select_files)
        
        top_layout = QHBoxLayout()
        top_layout.addWidget(self.output_button)
        top_layout.addWidget(self.reload_button)
        top_layout.addWidget(self.file_select_button)
        
        self.label = QLabel("Ziehe Dateien oder Ordner hierher oder nutze die Buttons oben.", self)
        self.label.setAlignment(Qt.AlignCenter)
        self.label.setWordWrap(True)
        
        self.file_list = QListWidget(self)
        self.file_list.setToolTip("Geladene Dateien")
        
        self.remove_button = QPushButton("Entfernen", self)
        self.remove_button.setToolTip("Ausgewählte Dateien entfernen.")
        self.remove_button.clicked.connect(self.remove_selected_files)
        
        self.process_button = QPushButton("Los", self)
        self.process_button.setToolTip("Verarbeitung starten.")
        self.process_button.clicked.connect(self.process_all_files)
        
        bottom_layout = QHBoxLayout()
        bottom_layout.addWidget(self.remove_button)
        bottom_layout.addWidget(self.process_button)
        
        self.progress_bar = QProgressBar(self)
        self.progress_bar.setValue(0)
        self.progress_bar.setVisible(False)
        
        main_layout = QVBoxLayout()
        main_layout.addLayout(top_layout)
        main_layout.addSpacing(10)
        main_layout.addWidget(self.label)
        main_layout.addSpacing(10)
        main_layout.addWidget(self.file_list)
        main_layout.addSpacing(10)
        main_layout.addLayout(bottom_layout)
        main_layout.addSpacing(10)
        main_layout.addWidget(self.progress_bar)
        
        self.setLayout(main_layout)
        
        self.resize(600, 400)
        
        self.file_paths = []
    
    def reload_labelcodes(self):
        self.label_dict = load_labelcodes(self.labelcodes_file)
        self.label.setText("Labelcodes wurden neu geladen.")
    
    def choose_output_directory(self):
        directory = QFileDialog.getExistingDirectory(self, "Ausgabeort wählen", self.output_dir)
        if directory:
            self.output_dir = directory
            self.label.setText(f"Ausgabeort: {self.output_dir}")
    
    def select_files(self):
        files, _ = QFileDialog.getOpenFileNames(self, "Dateien auswählen", "", "Text Files (*.txt)")
        if files:
            added_count = 0
            for f in files:
                if f not in self.file_paths:
                    self.file_paths.append(f)
                    self.file_list.addItem(f)
                    added_count += 1
            if self.file_paths:
                self.label.setText(f"{len(self.file_paths)} Datei(en) geladen. ({added_count} neu)")
            else:
                self.label.setText("Keine Dateien geladen.")
    
    def dragEnterEvent(self, event):
        if event.mimeData().hasUrls():
            event.acceptProposedAction()
        else:
            event.ignore()
    
    def dropEvent(self, event):
        from processing import list_txt_files_in_dir
        urls = event.mimeData().urls()
        if not urls:
            return
        added_count = 0
        for url in urls:
            file_path = url.toLocalFile()
            if file_path and not file_path in self.file_paths:
                if not file_path.lower().endswith('.txt') and not os.path.isdir(file_path):
                    # Nur txt oder Ordner
                    continue
                if os.path.isdir(file_path):
                    txt_files = list_txt_files_in_dir(file_path)
                    for tf in txt_files:
                        if tf not in self.file_paths:
                            self.file_paths.append(tf)
                            self.file_list.addItem(tf)
                            added_count += 1
                else:
                    self.file_paths.append(file_path)
                    self.file_list.addItem(file_path)
                    added_count += 1
        
        if self.file_paths:
            self.label.setText(f"{len(self.file_paths)} Datei(en) geladen. (+{added_count} neu)")
        else:
            self.label.setText("Keine Dateien geladen.")
    
    def remove_selected_files(self):
        selected_items = self.file_list.selectedItems()
        if not selected_items:
            self.label.setText("Keine Datei zum Entfernen ausgewählt.")
            return
        
        for item in selected_items:
            file_path = item.text()
            if file_path in self.file_paths:
                self.file_paths.remove(file_path)
            self.file_list.takeItem(self.file_list.row(item))
        
        if self.file_paths:
            self.label.setText(f"{len(self.file_paths)} Datei(en) verbleiben.")
        else:
            self.label.setText("Keine Dateien geladen.")
    
    def process_all_files(self):
        if not self.file_paths:
            self.label.setText("Keine Dateien geladen. Bitte erst Dateien hinzufügen.")
            return
        
        try:
            self.progress_bar.setVisible(True)
            self.progress_bar.setMinimum(0)
            self.progress_bar.setMaximum(len(self.file_paths))
            self.progress_bar.setValue(0)
            
            for i, input_file in enumerate(self.file_paths, start=1):
                summary = process_single_file(input_file, self.output_dir, self.label_dict, self.csv_columns)
                self.label.setText(summary)
                self.progress_bar.setValue(i)
            
            self.label.setText("Verarbeitung abgeschlossen. Siehe ggf. error.log für Details.")
            self.progress_bar.setVisible(False)
        except Exception as e:
            self.label.setText(f"Fehler beim Verarbeiten: {e}")
            log_error("Exception: " + traceback.format_exc())
            self.progress_bar.setVisible(False)
