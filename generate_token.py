
import jwt
import datetime

secret = "unused_in_demo" # Matching the .env value
# Or fetch from env if needed: import os; secret = os.getenv("SUPABASE_JWT_SECRET", "demo-secret")

payload = {
    "sub": "test-user-123",
    "role": "authenticated",
    "exp": datetime.datetime.utcnow() + datetime.timedelta(hours=1),
    "iat": datetime.datetime.utcnow(),
    "email": "test@example.com"
}

token = jwt.encode(payload, secret, algorithm="HS256")
print(token)
