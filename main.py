import sys
from PyQt5.QtWidgets import QApplication
from gui import DragDropWindow

if __name__ == '__main__':
    app = QApplication(sys.argv)
    window = DragDropWindow()
    window.show()
    sys.exit(app.exec_())
