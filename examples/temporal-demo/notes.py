import datetime
def save_note(text):
    with open("notes.txt", "a") as f:
        f.write(f"[{datetime.datetime.now()}] {text}\n")
    print("Note saved!")

if __name__ == "__main__":
    save_note("Hello from GnawTreeWriter!")