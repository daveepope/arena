CREATE TABLE weather_report (
    id NUMBER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    recorded_at TIMESTAMP NOT NULL,
    precipitation NUMBER NOT NULL,
    humidity NUMBER NOT NULL,
    pressure NUMBER NOT NULL
);
