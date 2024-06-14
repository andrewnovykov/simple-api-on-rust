```
curl -X POST http://localhost:8080/register -H "Content-Type: application/json" -d '{ "id":1, "email": "user@example.com", "password": "password123"}'
```

```
curl -X POST http://localhost:8080/login -H "Content-Type: application/json" -d '{"id": 1,  "email": "user@example.com", "password": "password123"}'
```

```
curl -X POST http://localhost:8080/items -H "Content-Type: application/json" -H "Authorization: Bearer token" -d '{"id": 1 , "name": "Item1", "price": 10.0}'
```

```
curl -X GET http://localhost:8080/items/1
curl -X GET http://localhost:8080/items
```

```
curl -X PUT http://localhost:8080/updateitems -H "Content-Type: application/json" -H "Authorization: Bearer token1" -d '{"ids": [1], "item": {"name": "UpdatedItem1", "price": 20.0}}'
```
