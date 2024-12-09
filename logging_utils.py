def log_error(message: str):
    with open('error.log', 'a', encoding='utf-8') as f:
        f.write(message + '\n')
