FROM python:3.11-slim

WORKDIR /app

RUN apt-get update && apt-get install -y \
    libsndfile1 \
    && rm -rf /var/lib/apt/lists/*

COPY ./autoeq ./autoeq/
COPY ./pyproject.toml .
COPY ./README.md .
COPY ./main.py .

RUN pip install --no-cache-dir hatchling && \
    pip install --no-cache-dir -e . && \
    pip install --no-cache-dir uvicorn fastapi python-multipart

EXPOSE 8000

CMD ["python", "main.py", "--host", "0.0.0.0", "--port", "8000"]