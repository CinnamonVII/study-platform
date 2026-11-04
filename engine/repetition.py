"""SM-2, quality grades 0..5. Schedule UTC calendar-day intervals."""
from datetime import datetime,timedelta,timezone
from engine.models import Review

def schedule(review:Review,grade:int,at=None):
    if not 0<=grade<=5: raise ValueError('Grade must be 0..5')
    at=at or datetime.now(timezone.utc)
    r=review.model_copy(deep=True)
    if grade<3: r.repetitions=0; r.interval=1
    else:
        r.interval=1 if r.repetitions==0 else 6 if r.repetitions==1 else round(r.interval*r.ease)
        r.repetitions+=1
    r.ease=max(1.3,r.ease+.1-(5-grade)*(.08+(5-grade)*.02))
    r.last_grade=grade; r.due=(at+timedelta(days=r.interval)).isoformat()
    return r
